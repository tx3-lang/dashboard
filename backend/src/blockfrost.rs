use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

use crate::db;
use crate::registry::Protocol;
use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct BlockfrostResponse {
    pub hash: String,
    pub inputs: Vec<BlockfrostInput>,
    pub outputs: Vec<BlockfrostOutput>,
}

#[derive(Debug, Deserialize)]
pub struct BlockfrostInput {
    pub address: String,
    pub tx_hash: String,
    pub output_index: u32,
    pub amount: Vec<BlockfrostAmount>,
    pub inline_datum: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockfrostOutput {
    pub address: String,
    pub output_index: u32,
    pub amount: Vec<BlockfrostAmount>,
    pub inline_datum: Option<String>,
    pub consumed_by_tx: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockfrostAmount {
    pub unit: String,
    pub quantity: String,
}

fn get_matching_protocol_tx(tx: &BlockfrostResponse, protocol: &Protocol) -> Option<String> {
    for protocol_tx in &protocol.txs {
        let inputs_match = protocol_tx.inputs.iter().all(|input| {
            tx.inputs.iter().any(|tx_input| {
                tx_input.address == *input
            })
        });

        let outputs_match = protocol_tx.outputs.iter().all(|output| {
            tx.outputs.iter().any(|tx_output| {
                tx_output.address == *output
            })
        });

        if inputs_match && outputs_match {
            return Some(protocol_tx.name.clone());
        }
    }

    None
}

async fn process_tx(
    config: &Config,
    http_client: &Client,
    sqlite_client: &sqlx::SqlitePool,
    protocol: &Protocol,
    tx_hash: &str
) -> Result<()> {
    let url = format!(
        "{}/txs/{}/utxos",
        config.blockfrost_url,
        tx_hash
    );

    let response = http_client
        .get(&url)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!(status = %status, body = %body, "Blockfrost query failed");
        return Ok(());
    }

    let blockfrost_response: BlockfrostResponse = response.json().await?;

    // TODO: Move the filtering logic to the utxorpc listener
    let tx_name = get_matching_protocol_tx(&blockfrost_response, protocol);
    if let Some(tx_name) = tx_name {
        tracing::info!(tx_hash = %tx_hash, tx_name = %tx_name, "Saving tx");
        db::insert_tx(sqlite_client, &tx_name, &blockfrost_response).await?;
    } else {
        tracing::info!(tx_hash = %tx_hash, "No matching protocol tx found");
    }

    Ok(())
}

pub async fn process_txs(
    config: &Config,
    sqlite_client: &sqlx::SqlitePool,
    protocol: &Protocol,
) -> Result<()> {
    let http_client = Client::new();

    for tx_hash in &config.txs {
        process_tx(config, &http_client, sqlite_client, protocol, tx_hash).await?;
    }

    Ok(())
}
