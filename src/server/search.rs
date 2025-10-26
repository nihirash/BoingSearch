use std::collections::HashMap;

use axum::response::{Html, IntoResponse};
use deunicode::deunicode;
use log::debug;
use serde::Deserialize;
use serpapi_search_rust::serp_api_search::SerpApiSearch;
use templr::{Template, templ, templ_ret};

use crate::server::freeserp::get_free_serp;

#[derive(Deserialize, Clone, Debug)]
struct SerpResult {
    pub organic_results: Vec<Serp>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct Serp {
    pub link: String,
    pub displayed_link: String,
    pub title: String,
    pub snippet: Option<String>,
}

#[derive(Clone)]
pub struct SearchRequester {
    pub api_key: String,
}

impl SearchRequester {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub async fn get_serp(&self, query: String, offset: Option<u32>) -> anyhow::Result<Vec<Serp>> {
        let mut params = HashMap::new();
        params.insert("engine".to_string(), "google".to_string());
        params.insert("q".to_string(), query);
        params.insert("num".to_string(), "10".to_string());

        if let Some(off) = offset
            && off > 0
        {
            params.insert("start".to_string(), off.to_string());
        }

        let search = SerpApiSearch::google(params, self.api_key.clone());
        let result = search
            .json()
            .await
            .map_err(|e| anyhow::anyhow!(format!("SERP Parsion error: {e}")))?;

        let result: SerpResult = serde_json::from_value(result)?;

        debug!("{result:#?}");

        Ok(result.organic_results)
    }

    pub async fn root_path_handler(
        &self,
        query_params: HashMap<String, String>,
    ) -> impl IntoResponse + use<> {
        let free = if query_params.contains_key("free") {
            "checked".to_string()
        } else {
            "".to_string()
        };

        match query_params.get("q") {
            Some(query) => {
                let offset = query_params.get("offset").and_then(|o| str::parse(o).ok());

                let serp_result = if free.is_empty() {
                    self.get_serp(query.clone(), offset)
                        .await
                        .unwrap_or(Vec::new())
                } else {
                    get_free_serp(query.clone()).await.unwrap_or(Vec::new())
                };

                let req_left = self.show_request_left().await;

                Self::build_serp_result_page(query.clone(), serp_result, req_left, free)
                    .render(&())
                    .map_err(|e| Html(format!("<h1>Error while building page</h1><p>{e}</p>")))
                    .map(Html)
            }
            None => Self::build_home_page()
                .render(&())
                .map_err(|e| Html(format!("<h1>Error while building page</h1><p>{e}</p>")))
                .map(Html),
        }
    }

    fn render_serp_item(serp_item: Serp) -> templ_ret!['static] {
        templ! {
            <a href={format!("/browse/?url={}", serp_item.link.clone())}>
                <h3>{serp_item.title}</h3>
                <h4>{serp_item.displayed_link}</h4>
            </a>
            <a href={serp_item.link}>[Full version]</a><br/>
            <small>
                {deunicode(&serp_item.snippet.clone().unwrap_or("".to_string()))}
            </small>
            <hr/>
        }
    }

    async fn show_request_left(&self) -> String {
        let r = SerpApiSearch::new("google".to_string(), HashMap::new(), self.api_key.clone());
        let acc = r.account().await;

        match acc {
            Ok(o) => {
                let queries = o
                    .get("total_searches_left")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0u64);

                format!("Left {queries} search queries on SerpAPI account")
            }
            Err(_) => "Cannot fetch info about SerpAPI accout state".to_string(),
        }
    }

    fn build_serp_result_page(
        query: String,
        serp_result: Vec<Serp>,
        req_left: String,
        free_search: String,
    ) -> templ_ret!['static] {
        templ! {
            <html>
            <head>
                <title>BoingSearch! {query} </title>
            </head>
            <body>
                <form action="/" method= "get">
                    <table widht="100%" border="0">
                        <tr widht="100%">
                            <td>
                                <a href="/"><img src="/static/logo.gif" alt="BoingSearch Logo" /></a>
                            </td>
                            <td>
                                    Search results for: <input type="text" size="30" name="q" value={query}/><br/><br/>
                                    #if free_search.is_empty() {
                                        <input type="checkbox" name="free" /> Use only free requests | <input type="submit" value="Search!"/><br/>
                                    } else {
                                        <input type="checkbox" name="free" checked /> Use only free requests | <input type="submit" value="Search!"/><br/>
                                    }
                            </td>
                        </tr>
                    </table>
                </form>

                <hr/>

                    #for item in &serp_result {
                        #Self::render_serp_item(item.clone());
                        <br/>
                    }

                    #if free_search.is_empty() {
                        <center>
                            #for n in 0..10 {
                                #let link = format!("/?q={query}&offset={}", n * 10);
                                <a href={link}>{n+1}</a>&nbsp;
                            }
                        </center>
                    }

                <center>
                    <b>{req_left}</b>
                </center>

                #Self::build_footer();
            </body>
            </html>
        }
    }

    fn build_home_page() -> templ_ret!['static] {
        templ! {
            <html>
            <head>
                <title>BoingSearch!</title>
            </head>
            <body>

                <br/>
                <br/>
                <center><img src="/static/logo.gif" alt="BoingSearch Logo"/></center>

                <center>
                    <h2>The Search Engine for Amigians and Friends</h2>
                    <h3>And web page <a href="/browse/">simplificator</a></h3>
                </center>

                <center>
                    <form action="/" method="get">
                    I am looking for: <br/>
                        <input type="text" size="30" name="q"/> <br/>
                        <input type="checkbox" name="free" checked /> Use only free requests(Bypass paid SerpAPI) <br/>
                        <small>Free search uses duckduckgo engine, SerpAPI 's one - uses Google</small> <br/>
                        <input type="submit" value="Search!"/>
                    </form>
                </center>

                #Self::build_footer();
            </body>
            </html>
        }
    }

    fn build_footer() -> templ_ret!['static] {
        templ! {
                <br/>
                <br/>
                <center>Inspired by FrogFing by ActionRetro, Recreated from scratch by <b>Nihirash</b></center>
                <center>You can <a href="/static/support.html">support this project</a> with donations!</center>
                <center>Powered by SerpAPI</center>

        }
    }
}
