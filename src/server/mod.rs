pub mod search;
pub mod simplifier;

use std::collections::HashMap;
use std::sync::Arc;

use axum::Extension;
use axum::extract::Query;
use axum::response::{Html, IntoResponse};
use axum::{Router, routing::get};
use log::info;
use serpapi_search_rust::serp_api_search::SerpApiSearch;
use templr::Template;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use url::Url;

use crate::AppConfig;
use crate::server::search::view::{build_error_page, build_home_page, serp_result_page};
use crate::server::search::{SearchEngine, SearchProvider};
use crate::server::simplifier::{process_page, proxy_page};

#[derive(Clone)]
pub struct Server<A: SearchProvider, B: SearchProvider> {
    pub host: String,
    pub port: u16,
    pub search_service: Arc<SearchEngine<A, B>>,
    pub base_path: Url,
    pub api_key: String,
}

#[derive(Clone)]
pub struct Context<A: SearchProvider, B: SearchProvider> {
    pub search_service: Arc<SearchEngine<A, B>>,
    pub base_path: String,
    pub serpapi: Arc<SerpApiSearch>,
}

impl<A: SearchProvider, B: SearchProvider> Server<A, B> {
    pub fn new(app_config: AppConfig, search_service: SearchEngine<A, B>) -> Self {
        Server {
            host: app_config.host.clone(),
            port: app_config.port,
            search_service: Arc::new(search_service.clone()),
            base_path: app_config.base_path.clone(),
            api_key: app_config.api_key,
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let context = Arc::new(Context {
            search_service: self.search_service.clone(),
            base_path: self.base_path.to_string(),
            serpapi: Arc::new(serpapi_search_rust::serp_api_search::SerpApiSearch::new(
                "google".to_string(),
                HashMap::new(),
                self.api_key.clone(),
            )),
        });

        let router: Router = axum::Router::new()
            .nest_service(
                "/static",
                ServeDir::new("assets/static/")
                    .not_found_service(ServeFile::new("assets/404.html")),
            )
            .route_service("/browse/", get(Self::browse_handler))
            .route_service("/next/", get(Self::next_page_handler))
            .route_service("/", get(Self::root_path_handler))
            .fallback_service(ServeFile::new("assets/404.html"))
            .layer(TraceLayer::new_for_http())
            .layer(Extension(context));

        let listener =
            tokio::net::TcpListener::bind(format!("{}:{}", self.host, self.port)).await?;

        info!("Started web server: http://{}:{}", self.host, self.port);

        axum::serve(listener, router.into_make_service()).await?;

        Ok(())
    }

    async fn browse_handler(
        Query(query_params): Query<HashMap<String, String>>,
        Extension(ext): Extension<Arc<Context<A, B>>>,
    ) -> impl IntoResponse {
        let url = query_params.get("url").cloned().unwrap_or("".to_string());

        let ext = Arc::clone(&ext);

        let content = if url.is_empty() {
            "<h1>Welcome to BoingSearch Simplifier!</h1><p>Enter url and press 'GO' button</p>"
                .to_string()
        } else {
            match process_page(url.clone(), format!("{}browse/", ext.base_path.clone())).await {
                Ok(e) => e,
                Err(e) => format!("<h1>Error happens</h1><p>{e}</p>"),
            }
        };

        let result = match proxy_page(url, content).render(&()) {
            Ok(c) => c,
            Err(e) => format!("<h1>Error happens</h1><p>{e}</p>"),
        };

        Html(result)
    }

    async fn next_page_handler(
        Query(params): Query<HashMap<String, String>>,
        Extension(ext): Extension<Arc<Context<A, B>>>,
    ) -> impl IntoResponse {
        let ext = Arc::clone(&ext);

        let query = params.get("q").unwrap_or(&"".to_string()).clone();

        let result = ext
            .search_service
            .next_page(params)
            .await
            .and_then(|r| serp_result_page(query, r))
            .map(Html);

        match result {
            Ok(r) => r,
            Err(e) => build_error_page(e.to_string())
                .map(Html)
                .unwrap_or(Html("<h1>Internal error</h1>".to_string())),
        }
    }

    async fn root_path_handler(
        Query(query_params): Query<HashMap<String, String>>,
        Extension(ext): Extension<Arc<Context<A, B>>>,
    ) -> impl IntoResponse {
        let ext = Arc::clone(&ext);
        let q = query_params.get("q");
        let premium = query_params
            .get("premium")
            .unwrap_or(&"".to_string())
            .clone();

        let result = match q {
            Some(query) => {
                let result = ext
                    .search_service
                    .first_search(query.clone(), premium)
                    .await;

                result.and_then(|result| serp_result_page(query.clone(), result))
            }
            None => {
                let serpapi_left = ext
                    .serpapi
                    .account()
                    .await
                    .ok()
                    .and_then(|o| {
                        o.get("total_searches_left");
                        o.as_u64()
                    })
                    .unwrap_or(0u64);

                build_home_page(serpapi_left)
            }
        };

        match result {
            Ok(r) => Html(r),
            Err(e) => build_error_page(e.to_string())
                .map(Html)
                .unwrap_or(Html("<h1>Internal error</h1>".to_string())),
        }
    }
}
