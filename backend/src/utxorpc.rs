use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use pallas::codec::{
    minicbor,
    utils::{Bytes, NonEmptySet, KeepRaw},
};
use pallas::ledger::{
    addresses::Address,
    primitives::{PlutusData, conway::VKeyWitness},
    traverse::MultiEraTx,
};
use utxorpc::{
    ClientBuilder,
    CardanoQueryClient,
    CardanoSubmitClient,
    NativeBytes,
    spec::cardano::{AddressPattern, TxOutputPattern},
};

use crate::db;
use crate::registry::Protocol;
use crate::config::Config;

pub struct Listener {
    pub u5c_url: String,
    pub u5c_api_key: Option<String>,
}

impl Listener {
    pub async fn listen_txs(
        &self,
        config: &Config,
        sqlite_client: &sqlx::SqlitePool,
        protocol: &Protocol,
    ) -> Result<()> {
        let mut client_builder = ClientBuilder::new().uri(&self.u5c_url)?;

        if let Some(u5c_api_key) = &self.u5c_api_key {
            client_builder = client_builder.metadata("dmtr-api-key", u5c_api_key.clone())?;
        }

        let mut client = client_builder.build::<CardanoQueryClient>().await;

        // TODO: Implement listening to blockchain transactions

        Ok(())
    }
}
