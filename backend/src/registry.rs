use anyhow::{Context, Result};
use pallas::ledger::addresses::Address;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::Config;

#[derive(Debug, Serialize)]
struct RegistryRequest<'a> {
    query: &'a str,
    variables: RegistryVariables<'a>,
}

#[derive(Debug, Serialize)]
struct RegistryVariables<'a> {
    scope: &'a str,
    name: &'a str,
}

#[derive(Debug, Deserialize)]
struct RegistryResponse {
    data: Option<RegistryResponseData>,
    errors: Option<Vec<RegistryResponseError>>,
}

#[derive(Debug, Deserialize)]
struct RegistryResponseData {
    protocol: Option<RegistryProtocol>,
}

#[derive(Debug, Deserialize)]
struct RegistryProtocol {
    source: String,
}

#[derive(Debug, Deserialize)]
struct RegistryResponseError {
    message: String,
}

#[derive(Debug, Clone)]
pub struct Protocol {
    pub txs: Vec<ProtocolTx>,
}

#[derive(Debug, Clone)]
pub struct ProtocolTx {
    pub name: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub mints: Vec<ProtocolTxMint>,
}


#[derive(Debug, Clone)]
pub struct ProtocolTxMint {
    pub policy: String,
}

fn parse_type_name(type_name: &str) -> Result<tx3_resolver::Type> {
    use tx3_resolver::Type;

    let ty = match type_name {
        "Undefined" => Type::Undefined,
        "Unit" => Type::Unit,
        "Int" => Type::Int,
        "Bool" => Type::Bool,
        "Bytes" => Type::Bytes,
        "Address" => Type::Address,
        "Utxo" => Type::Utxo,
        "UtxoRef" => Type::UtxoRef,
        "AnyAsset" => Type::AnyAsset,
        "List" => Type::List,
        "Map" => Type::Map,
        other => Type::Custom(other.to_string()),
    };

    Ok(ty)
}

fn map_protocol_tx(config: &Config, env: &tx3_tir::reduce::ArgMap, tx: &tx3_lang::ast::TxDef) -> Result<ProtocolTx> {
    let tx_name = tx.name.value.clone();

    let parameter_types: HashMap<String, String> = tx
        .parameters
        .parameters
        .iter()
        .map(|param| (param.name.value.clone(), param.r#type.to_string()))
        .collect();

    let mut parameters = tx3_tir::reduce::ArgMap::new();
    if let Some(txs) = config.protocol_parameters.get("txs") {
        if let Some(tx_params) = txs.get(&tx_name) {
            for (key, value) in tx_params.as_object().context("Protocol parameters must be a JSON object")? {
                if let Some(type_name) = parameter_types.get(key) {
                    let arg_value = tx3_resolver::interop::from_json(value.clone(), &parse_type_name(type_name)?)?;
                    parameters.insert(key.to_string(), arg_value);
                }
            }
        }
    }

    let lower = tx3_lang::lowering::lower_tx(tx)?;
    
    let mut inputs = Vec::new();
    for input in &lower.inputs {
        if let tx3_tir::model::v1beta0::Expression::EvalParam(param) = &input.utxos {
            if let tx3_tir::model::v1beta0::Param::ExpectInput(_, query) = param.as_ref() {
                if let tx3_tir::model::v1beta0::Expression::EvalParam(param) = &query.address {
                    if let tx3_tir::model::v1beta0::Param::ExpectValue(input_name, _) = param.as_ref() {
                        if let Some(value) = env.get(input_name) {
                            if let tx3_tir::reduce::ArgValue::Address(bytes) = value {
                                let address = Address::from_bytes(bytes)?;
                                inputs.push(address.to_bech32()?);
                            }
                        } else if let Some(value) = parameters.get(input_name) {
                            if let tx3_tir::reduce::ArgValue::Address(bytes) = value {
                                let address = Address::from_bytes(bytes)?;
                                inputs.push(address.to_bech32()?);
                            }
                        }
                    }
                }
            }
        }
    }
    
    let mut outputs = Vec::new();
    for output in &lower.outputs {
        if let tx3_tir::model::v1beta0::Expression::EvalParam(param) = &output.address {
            if let tx3_tir::model::v1beta0::Param::ExpectValue(output_name, _) = param.as_ref() {
                if let Some(value) = env.get(output_name) {
                    if let tx3_tir::reduce::ArgValue::Address(bytes) = value {
                        let address = Address::from_bytes(bytes)?;
                        outputs.push(address.to_bech32()?);
                    }
                } else if let Some(value) = parameters.get(output_name) {
                    if let tx3_tir::reduce::ArgValue::Address(bytes) = value {
                        let address = Address::from_bytes(bytes)?;
                        outputs.push(address.to_bech32()?);
                    }
                }
            }
        }
    }

    let mut mints = Vec::new();
    for mint in &lower.mints {
        if let tx3_tir::model::v1beta0::Expression::Assets(assets) = &mint.amount {
            for asset in assets {
                if let tx3_tir::model::v1beta0::Expression::EvalParam(param) = &asset.policy {
                    if let tx3_tir::model::v1beta0::Param::ExpectValue(policy_name, _) = param.as_ref() {
                        if let Some(value) = env.get(policy_name) {
                            if let tx3_tir::reduce::ArgValue::Bytes(bytes) = value {
                                mints.push(ProtocolTxMint { policy: hex::encode(bytes) });
                            }
                        } else if let Some(value) = parameters.get(policy_name) {
                            if let tx3_tir::reduce::ArgValue::Bytes(bytes) = value {
                                mints.push(ProtocolTxMint { policy: hex::encode(bytes) });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ProtocolTx {
        name: tx_name,
        inputs,
        outputs,
        mints,
    })
}

pub async fn fetch_protocol(
    config: &Config,
    protocol_scope: &str,
    protocol_name: &str,
) -> Result<Protocol> {
    let client = Client::new();

    let request = RegistryRequest {
        query: "query($scope: String!, $name: String!) { protocol(scope: $scope, name: $name) { source } }",
        variables: RegistryVariables {
            scope: protocol_scope,
            name: protocol_name,
        },
    };

    let response = client
        .post(&config.registry_url)
        .json(&request)
        .send()
        .await
        .context("Registry request failed")?;

    let payload: RegistryResponse = response.json().await.context("Invalid JSON response")?;

    if let Some(errors) = payload.errors {
        let message = errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(anyhow::anyhow!("Registry error: {message}"));
    }

    let source = payload
        .data
        .and_then(|data| data.protocol)
        .map(|protocol| protocol.source)
        .context("Missing protocol source in registry response")?;

    let mut program = tx3_lang::parsing::parse_string(&source)?;

    tx3_lang::analyzing::analyze(&mut program).ok()?;

    let env = program
        .env
        .and_then(|env| {
            Some(env.fields.iter()
                .map(|field| (field.name.clone(), field.r#type.to_string()))
                .collect::<HashMap<String, String>>())
        })
        .unwrap_or_default();

    let mut parameters = tx3_tir::reduce::ArgMap::new();
    if let Some(env_params) = config.protocol_parameters.get("env") {
        for (key, value) in env_params.as_object().context("Protocol parameters must be a JSON object")? {
            if let Some(type_name) = env.get(key) {
                let arg_value = tx3_resolver::interop::from_json(value.clone(), &parse_type_name(type_name)?)?;
                parameters.insert(key.to_string(), arg_value);
            }
        }
    }
    if let Some(parties_values) = config.protocol_parameters.get("parties") {
        for (key, value) in parties_values.as_object().context("Protocol parameters must be a JSON object")? {
            let arg_value = tx3_resolver::interop::from_json(value.clone(), &tx3_resolver::Type::Address)?;
            parameters.insert(key.to_string(), arg_value);
        }
    }

    let txs = program
        .txs
        .iter()
        .map(|tx| map_protocol_tx(&config, &parameters, tx))
        .collect::<Result<Vec<_>>>()?;

    Ok(Protocol { txs })
}
