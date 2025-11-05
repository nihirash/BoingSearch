pub mod duckduckprovider;
pub mod serpapiprovider;
pub mod view;

use censor::Censor;
use serde::Deserialize;

use log::{info, warn};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Debug)]
pub struct Serp {
    pub link: String,
    pub displayed_link: String,
    pub title: String,
    pub snippet: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SearchResponse {
    pub serp: Vec<Serp>,
    pub inputs: HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait SearchProvider: Clone + Sync + Send + 'static {
    /// Make first search request
    async fn make_serp_request(&self, query: String) -> anyhow::Result<SearchResponse>;

    /// Pagination to next page
    async fn next_page(&self, inputs: HashMap<String, String>) -> anyhow::Result<SearchResponse>;
}

#[derive(Clone)]
pub struct SearchEngine<A: SearchProvider, B: SearchProvider> {
    pub free: A,
    pub premium: B,
    pub censor: Censor,
}

impl<A: SearchProvider, B: SearchProvider> SearchEngine<A, B> {
    pub fn new(free: A, premium: B) -> Self {
        let censor_words = include_str!("../../../assets/censorwords.txt").lines();
        let mut censor = Censor::Sex + Censor::Standard;
        for word in censor_words {
            censor = censor + word;
        }

        Self {
            free,
            premium,
            censor,
        }
    }

    pub async fn first_search(
        &self,
        query: String,
        premium: String,
    ) -> anyhow::Result<SearchResponse> {
        if self.censor.check(&query) {
            anyhow::bail!("Your request was denied by internal rules");
        }

        if premium.is_empty() {
            match self.free.make_serp_request(query.clone()).await {
                Ok(r) => Ok(r),
                Err(e) => {
                    warn!("Error during premium search: {e}");

                    self.premium.make_serp_request(query).await
                }
            }
        } else {
            match self.premium.make_serp_request(query.clone()).await {
                Ok(r) => Ok(r),
                Err(e) => {
                    warn!("Error during premium search: {e}");

                    self.free.make_serp_request(query).await
                }
            }
        }
    }

    pub async fn next_page(
        &self,
        inputs: HashMap<String, String>,
    ) -> anyhow::Result<SearchResponse> {
        if inputs.contains_key("premium") {
            info!("Premium next page");
            self.premium.next_page(inputs).await
        } else {
            info!("Free next page");
            self.free.next_page(inputs).await
        }
    }
}
