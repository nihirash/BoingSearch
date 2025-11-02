use crate::server::search::{SearchProvider, SearchResponse, Serp};
use log::debug;
use serde::Deserialize;
use serpapi_search_rust::serp_api_search::SerpApiSearch;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct SerpApiProvider {
    pub api_key: String,
}

#[derive(Deserialize, Clone, Debug)]
struct SerpResult {
    pub organic_results: Vec<Serp>,
}

impl SerpApiProvider {
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
}

#[async_trait::async_trait]
impl SearchProvider for SerpApiProvider {
    async fn make_serp_request(&self, query: String) -> anyhow::Result<SearchResponse> {
        let serp = self.get_serp(query.clone(), None).await?;

        let mut inputs = HashMap::new();
        inputs.insert("q".to_string(), query);
        inputs.insert("premium".to_string(), "checked".to_string());
        inputs.insert("offset".to_string(), "10".to_string());

        Ok(SearchResponse { serp, inputs })
    }

    async fn next_page(&self, inputs: HashMap<String, String>) -> anyhow::Result<SearchResponse> {
        let query = inputs
            .get("q")
            .ok_or(anyhow::anyhow!("Query not provided!"))?
            .clone();
        let offset = inputs
            .get("offset")
            .ok_or(anyhow::anyhow!("Offset not provided"))?
            .clone();
        let offset: u32 = str::parse(&offset)?;

        let serp = self.get_serp(query.clone(), Some(offset)).await?;

        let mut inputs = HashMap::new();
        inputs.insert("q".to_string(), query);
        inputs.insert("offset".to_string(), (offset + 10).to_string());
        inputs.insert("premium".to_string(), "checked".to_string());

        Ok(SearchResponse { serp, inputs })
    }
}
