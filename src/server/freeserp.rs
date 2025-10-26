use crate::server::search::Serp;
use duckduckgo::{browser::Browser, user_agents};
use rand::prelude::IteratorRandom;
use reqwest::redirect::Policy;

pub async fn get_free_serp(query: String) -> anyhow::Result<Vec<Serp>> {
    let user_agent = user_agents::USER_AGENTS
        .values()
        .choose(&mut rand::rng())
        .cloned()
        .unwrap_or(crate::USER_AGENT);

    let req = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(user_agent)
        .redirect(Policy::limited(10))
        .build()?;

    let _ = req.get("http://lite.duckduckgo.com/").send().await?;
    let browser = Browser::new(req);

    let r = browser
        .lite_search(&query, "wt-WT", None, user_agent)
        .await?;

    let result: Vec<_> = r
        .into_iter()
        .map(|r| Serp {
            link: r.url.clone(),
            displayed_link: r.url.clone(),
            title: r.title.clone(),
            snippet: Some(r.snippet.clone()),
        })
        .collect();

    Ok(result)
}

#[tokio::test]
async fn call_get_free_serp() -> anyhow::Result<()> {
    let res = get_free_serp("Amiga 40".to_string()).await?;

    println!("{res:#?}");

    Ok(())
}
