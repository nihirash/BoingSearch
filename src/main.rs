use boing_search::{
    AppConfig,
    server::{
        Server,
        search::{
            SearchEngine, duckduckprovider::DuckDuckRequester, serpapiprovider::SerpApiProvider,
        },
    },
};
use log::{debug, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    colog::init();
    info!("BoingSearch started");
    debug!("Debug level active!");

    let app_config = AppConfig::try_create()?;

    debug!("Starting with config: {app_config:?}");

    let free = DuckDuckRequester::new(app_config.rate_limit, app_config.proxies.clone());
    let premium = SerpApiProvider::new(app_config.api_key.clone());

    let search_engine = SearchEngine::new(free, premium);

    Server::new(app_config, search_engine).start().await?;

    Ok(())
}
