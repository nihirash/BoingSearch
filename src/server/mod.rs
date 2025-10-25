pub mod search;
pub mod simplifier;

use std::collections::HashMap;
use std::sync::Arc;

use axum::Extension;
use axum::extract::Query;
use axum::response::{Html, IntoResponse};
use axum::{Router, routing::get};
use templr::Template;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use url::Url;

use crate::AppConfig;
use crate::server::search::SearchRequester;
use crate::server::simplifier::{process_page, proxy_page};

#[derive(Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub search_requester: Arc<SearchRequester>,
    pub base_path: Url,
}

#[derive(Clone)]
pub struct Context {
    pub search_requester: Arc<SearchRequester>,
    pub base_path: String,
}

impl Server {
    pub fn new(app_config: AppConfig, search_requester: SearchRequester) -> Self {
        Server {
            host: app_config.host.clone(),
            port: app_config.port,
            search_requester: Arc::new(search_requester),
            base_path: app_config.base_path.clone(),
        }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let context = Arc::new(Context {
            search_requester: self.search_requester.clone(),
            base_path: self.base_path.to_string(),
        });

        let router: Router = axum::Router::new()
            .nest_service(
                "/static",
                ServeDir::new("assets/static/")
                    .not_found_service(ServeFile::new("assets/404.html")),
            )
            .route_service("/browse/", get(Self::browse_handler))
            .route_service("/", get(Self::root_path_handler))
            .fallback_service(ServeFile::new("assets/404.html"))
            .layer(TraceLayer::new_for_http())
            .layer(Extension(context));

        let listener =
            tokio::net::TcpListener::bind(format!("{}:{}", self.host, self.port)).await?;
        axum::serve(listener, router.into_make_service()).await?;

        Ok(())
    }

    async fn browse_handler(
        Query(query_params): Query<HashMap<String, String>>,
        Extension(ext): Extension<Arc<Context>>,
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

    async fn root_path_handler(
        Query(query_params): Query<HashMap<String, String>>,
        Extension(ext): Extension<Arc<Context>>,
    ) -> impl IntoResponse + use<> {
        let ext = Arc::clone(&ext);

        ext.search_requester.root_path_handler(query_params).await
    }
}
