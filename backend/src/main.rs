mod api;
mod config;
mod db;
mod registry;
mod utxorpc;
mod blockfrost;

use anyhow::Result;
use axum::{routing::get, Router};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = config::Config::from_env()?;

    let protocol = registry::fetch_protocol(
        &config,
        &config.protocol_scope,
        &config.protocol_name,
    )
    .await?;

    dbg!(&protocol);

    let connect_options = SqliteConnectOptions::new()
        .filename(&config.database_path)
        .create_if_missing(true);

    let sqlite_client = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    db::init_db(&sqlite_client).await?;

    // TODO: Implement listening to blockchain transactions
    // let utxorpc_listener = utxorpc::Listener {
    //     u5c_url: config.u5c_url.clone(),
    //     u5c_api_key: config.u5c_api_key.clone(),
    // };
    // let utxorpc_listener_sqlite_client = sqlite_client.clone();
    // let utxorpc_listener_config = config.clone();
    // tokio::spawn(async move {
    //     if let Err(error) = utxorpc_listener
    //         .listen_txs(
    //             &utxorpc_listener_config,
    //             &utxorpc_listener_sqlite_client,
    //             &protocol,
    //         )
    //         .await
    //     {
    //         tracing::error!(error = %error, "Listener failed");
    //     }
    // });

    // TODO: Remove fixed transactions processing
    blockfrost::process_txs(&config, &sqlite_client, &protocol).await?;

    let api_state = api::ApiState {
        sqlite_client,
    };
    let api_router = Router::new()
        .route("/txs", get(api::list_txs))
        .route("/txs/:hash", get(api::get_tx))
        .with_state(api_state);

    let listener = tokio::net::TcpListener::bind(&config.server_addr).await?;
    tracing::info!(addr = %config.server_addr, "HTTP server listening");
    axum::serve(listener, api_router).await?;

    Ok(())
}
