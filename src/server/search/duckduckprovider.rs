use crate::server::search::SearchProvider;
use crate::server::search::Serp;
use chrono::Utc;
use itertools::Itertools;
use kuchiki::ElementData;
use kuchiki::NodeDataRef;
use log::{info, warn};
use reqwest::Client;
use reqwest::Proxy;
use reqwest::header::ACCEPT;
use reqwest::header::ACCEPT_LANGUAGE;
use reqwest::header::CONNECTION;
use reqwest::header::HeaderMap;
use reqwest::redirect::Policy;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicUsize};

use kuchiki::parse_html;
use kuchiki::traits::*;

use crate::server::search::SearchResponse;

#[derive(Clone, Debug)]
pub struct DuckDuckRequester {
    pub last_access_time: Arc<AtomicI64>,
    pub req_spacing_secs: i64,
    pub proxies: Vec<String>,
    pub proxy_counter: Arc<AtomicUsize>,
}

impl DuckDuckRequester {
    pub fn new(req_spacing_secs: i64, proxies: Vec<String>) -> DuckDuckRequester {
        Self {
            last_access_time: Arc::new(AtomicI64::new(Utc::now().timestamp())),
            req_spacing_secs,
            proxies,
            proxy_counter: Arc::new(AtomicUsize::new(Utc::now().timestamp() as usize)),
        }
    }

    fn try_extract_serp(data: Vec<NodeDataRef<ElementData>>) -> anyhow::Result<Serp> {
        let first_line = data.first().ok_or(anyhow::anyhow!("Data line absent"))?;
        let snippet_line = data.get(1).ok_or(anyhow::anyhow!("Snippet line absent"))?;
        let url_line = data.get(2).ok_or(anyhow::anyhow!("Url line absent"))?;

        let display_url = url_line.text_contents();
        let snippet = snippet_line.text_contents();

        let link = first_line
            .as_node()
            .select_first("a")
            .map_err(|_| anyhow::anyhow!("No link present"))?;
        let head_text = link.text_contents();
        let link_attrs = link.attributes.borrow();
        let dd_url = link_attrs
            .get("href")
            .ok_or(anyhow::anyhow!("No href in link"))?;

        let url = url::Url::from_str(&format!("http:{dd_url}"))?;
        let query = url
            .query_pairs()
            .into_owned()
            .collect::<HashMap<String, String>>();

        let target = query.get("uddg").cloned().unwrap_or(dd_url.to_string());

        let target = urlencoding::decode(&target)?.to_string();

        Ok(Serp {
            link: target,
            displayed_link: display_url,
            title: head_text,
            snippet: Some(snippet),
        })
    }

    fn parse_serp_result(page_txt: String) -> anyhow::Result<SearchResponse> {
        let page = parse_html().one(page_txt.clone());
        let mut serp_items: Vec<Serp> = Vec::new();
        let mut hidden_inputs: HashMap<String, String> = HashMap::new();

        if let Ok(inputs) = page.select("form.next_form>input[type=hidden]") {
            let inputs = inputs.collect::<Vec<_>>();
            for input in inputs {
                let attrs = input.attributes.borrow();
                if let Some(name) = attrs.get("name")
                    && let Some(v) = attrs.get("value")
                {
                    hidden_inputs.insert(name.to_string(), v.to_string());
                }
            }
        } else {
            warn!("No hidden input on serp result page!");
        }

        if let Ok(tables) = page.select("table") {
            for table in tables {
                if table.as_node().select("a.result-link").is_ok() {
                    info!("Found!!");

                    if let Ok(items) = table.as_node().select("tr") {
                        let items = items.chunks(4);
                        for item in &items {
                            let item = item.collect::<Vec<_>>();

                            match Self::try_extract_serp(item) {
                                Ok(serp) => serp_items.push(serp),
                                Err(e) => warn!("Error happens: {e:?}"),
                            }
                        }
                    }
                }
            }
        } else {
            anyhow::bail!("Cannot find tables on page");
        }

        Ok(SearchResponse {
            serp: serp_items,
            inputs: hidden_inputs,
        })
    }

    fn build_client(&self) -> anyhow::Result<Client> {
        let mut headers = HeaderMap::new();
        headers.append(ACCEPT, "text/html".parse()?);
        headers.append(ACCEPT_LANGUAGE, "en-US,en;q=0.9".parse()?);
        headers.append(CONNECTION, "close".parse()?);

        let idx = self
            .proxy_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let proxy = self
            .proxies
            .get(idx % self.proxies.len())
            .ok_or(anyhow::anyhow!("Proxy required"))?;

        info!("Using proxy: {proxy}");

        let client = reqwest::Client::builder()
            .cookie_store(false)
            .http1_only()
            .redirect(Policy::limited(30))
            .default_headers(headers)
            .user_agent(crate::USER_AGENT)
            .proxy(Proxy::http(proxy)?)
            .build()?;

        Ok(client)
    }

    async fn make_serp_request_inner(&self, query: String) -> anyhow::Result<SearchResponse> {
        let query = urlencoding::encode(&query).to_string();

        let client = self.build_client()?;

        let result = client
            .get(format!(
                "http://lite.duckduckgo.com/lite/?kl=wt-wt&q={query}"
            ))
            .send()
            .await?;
        let page_txt = result.text().await?;

        Self::parse_serp_result(page_txt)
    }

    async fn wait(&self) {
        let last_access = self
            .last_access_time
            .load(std::sync::atomic::Ordering::Acquire);

        let now = Utc::now().timestamp();
        let diff = now - (last_access);

        if diff < self.req_spacing_secs {
            info!("Meet request timeout");

            tokio::time::sleep(tokio::time::Duration::from_secs(
                (self.req_spacing_secs - diff) as u64,
            ))
            .await;
        }

        self.last_access_time
            .store(Utc::now().timestamp(), std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl SearchProvider for DuckDuckRequester {
    async fn make_serp_request(&self, query: String) -> anyhow::Result<SearchResponse> {
        self.wait().await;
        self.make_serp_request_inner(query).await
    }

    async fn next_page(&self, inputs: HashMap<String, String>) -> anyhow::Result<SearchResponse> {
        self.wait().await;
        let client = self.build_client()?;

        let r = client
            .post("https://lite.duckduckgo.com/lite/")
            .form(&inputs)
            .send()
            .await?;

        let page_txt = r.text().await?;

        Self::parse_serp_result(page_txt)
    }
}

#[tokio::test]
async fn test_new_free_serp_provider() -> anyhow::Result<()> {
    colog::init();

    let app_conf = crate::AppConfig::try_create()?;

    let provider = DuckDuckRequester::new(1, app_conf.proxies);

    let result = provider
        .make_serp_request("Amiga 40 reddit".to_string())
        .await?;

    info!("{result:#?}");

    let result = provider.next_page(result.inputs).await;

    info!("{result:#?}");

    Ok(())
}
