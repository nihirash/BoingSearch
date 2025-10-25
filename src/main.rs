use boing_search::server::search::SearchRequester;
use boing_search::{AppConfig, server::Server};
use log::{debug, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    colog::init();
    info!("BoingSearch started");
    debug!("Debug level active!");

    let app_config = AppConfig::try_create()?;

    debug!("Starting with config: {app_config:?}");

    let search_requester = SearchRequester::new(app_config.api_key.clone());

    Server::new(app_config, search_requester).start().await?;

    Ok(())
}
