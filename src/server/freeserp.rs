use crate::server::search::Serp;
use duckduckgo::browser::Browser;

pub async fn get_free_serp(query: String) -> anyhow::Result<Vec<Serp>> {
    let req = reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(crate::USER_AGENT)
        .build()?;

    let browser = Browser::new(req);

    let r = browser
        .lite_search(&query, "wt-WT", None, crate::USER_AGENT)
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
    let res = get_free_serp("Nihirash".to_string()).await?;

    println!("{res:#?}");

    Ok(())
}
