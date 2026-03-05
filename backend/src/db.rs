use anyhow::{Context, Result};
use sqlx::{QueryBuilder, SqlitePool};
use std::collections::{HashMap, HashSet};

use crate::blockfrost::BlockfrostResponse;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TxResponse {
    pub hash: String,
    pub tx_name: String,
    pub inputs: Vec<Utxo>,
    pub outputs: Vec<Utxo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Utxo {
    pub address: String,
    pub tx_hash: String,
    pub output_index: u32,
    pub amount: Vec<Amount>,
    pub datum: Option<String>,
    pub consumed_by_tx: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Amount {
    pub unit: String,
    pub quantity: String,
}

#[derive(Debug, sqlx::FromRow)]
struct TxRow {
    pub hash: String,
    pub tx_name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct UtxoRow {
    pub tx_hash: String,
    pub output_index: i64,
    pub address: String,
    pub consumed_by_tx: Option<String>,
    pub datum: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct AmountRow {
    pub tx_hash: String,
    pub output_index: i64,
    pub unit: String,
    pub quantity: String,
}

pub async fn init_db(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS txs (
            hash TEXT PRIMARY KEY,
            tx_name TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS utxos (
            tx_hash TEXT NOT NULL,
            output_index INTEGER NOT NULL,
            address TEXT NOT NULL,
            consumed_by_tx TEXT,
            datum TEXT,
            PRIMARY KEY (tx_hash, output_index)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS utxo_amount (
            tx_hash TEXT NOT NULL,
            output_index INTEGER NOT NULL,
            unit TEXT NOT NULL,
            quantity TEXT NOT NULL,
            PRIMARY KEY (tx_hash, output_index, unit)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_tx(pool: &SqlitePool, tx_name: &str, response: &BlockfrostResponse) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO txs (hash, tx_name)
        VALUES (?, ?)
        ON CONFLICT (hash) DO NOTHING
        "#,
    )
    .bind(&response.hash)
    .bind(tx_name)
    .execute(&mut *tx)
    .await?;

    for input in &response.inputs {
        sqlx::query(
            r#"
            INSERT INTO utxos (tx_hash, output_index, address, consumed_by_tx, datum)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (tx_hash, output_index) DO UPDATE SET
                address = excluded.address,
                consumed_by_tx = COALESCE(utxos.consumed_by_tx, excluded.consumed_by_tx),
                datum = COALESCE(utxos.datum, excluded.datum)
            "#,
        )
        .bind(&input.tx_hash)
        .bind(i64::from(input.output_index))
        .bind(&input.address)
        .bind(&response.hash)
        .bind(&input.inline_datum)
        .execute(&mut *tx)
        .await?;

        for amount in &input.amount {
            sqlx::query(
                r#"
                INSERT INTO utxo_amount (tx_hash, output_index, unit, quantity)
                VALUES (?, ?, ?, ?)
                ON CONFLICT (tx_hash, output_index, unit) DO UPDATE SET
                    quantity = excluded.quantity
                "#,
            )
            .bind(&input.tx_hash)
            .bind(i64::from(input.output_index))
            .bind(&amount.unit)
            .bind(&amount.quantity)
            .execute(&mut *tx)
            .await?;
        }
    }

    for output in &response.outputs {
        sqlx::query(
            r#"
            INSERT INTO utxos (tx_hash, output_index, address, consumed_by_tx, datum)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT (tx_hash, output_index) DO UPDATE SET
                address = excluded.address,
                consumed_by_tx = COALESCE(utxos.consumed_by_tx, excluded.consumed_by_tx),
                datum = COALESCE(utxos.datum, excluded.datum)
            "#,
        )
        .bind(&response.hash)
        .bind(i64::from(output.output_index))
        .bind(&output.address)
        .bind(&output.consumed_by_tx)
        .bind(&output.inline_datum)
        .execute(&mut *tx)
        .await?;

        for amount in &output.amount {
            sqlx::query(
                r#"
                INSERT INTO utxo_amount (tx_hash, output_index, unit, quantity)
                VALUES (?, ?, ?, ?)
                ON CONFLICT (tx_hash, output_index, unit) DO UPDATE SET
                    quantity = excluded.quantity
                "#,
            )
            .bind(&response.hash)
            .bind(i64::from(output.output_index))
            .bind(&amount.unit)
            .bind(&amount.quantity)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(())
}

pub async fn list_txs(pool: &SqlitePool, limit: i64) -> Result<Vec<TxResponse>> {
    let rows: Vec<TxRow> = sqlx::query_as(
        r#"
        SELECT hash, tx_name
        FROM txs
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    let hashes: Vec<String> = rows.iter().map(|row| row.hash.clone()).collect();
    let utxos = fetch_utxos(pool, &hashes).await?;
    let amounts = fetch_amounts(pool, &utxos).await?;

    build_responses(rows, utxos, amounts)
}

pub async fn get_tx(pool: &SqlitePool, tx_hash: &str) -> Result<Option<TxResponse>> {
    let row: Option<TxRow> = sqlx::query_as(
        r#"
        SELECT hash, tx_name
        FROM txs
        WHERE hash = ?
        "#,
    )
    .bind(tx_hash)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let hashes = vec![row.hash.clone()];
    let utxos = fetch_utxos(pool, &hashes).await?;
    let amounts = fetch_amounts(pool, &utxos).await?;
    let mut responses = build_responses(vec![row], utxos, amounts)?;

    Ok(responses.pop())
}

async fn fetch_utxos(pool: &SqlitePool, hashes: &[String]) -> Result<Vec<UtxoRow>> {
    if hashes.is_empty() {
        return Ok(vec![]);
    }

    let mut rows = fetch_utxos_by_column(pool, "tx_hash", hashes).await?;
    rows.extend(fetch_utxos_by_column(pool, "consumed_by_tx", hashes).await?);

    let mut seen = HashSet::new();
    rows.retain(|row| seen.insert((row.tx_hash.clone(), row.output_index)));

    Ok(rows)
}

async fn fetch_utxos_by_column(
    pool: &SqlitePool,
    column: &str,
    hashes: &[String],
) -> Result<Vec<UtxoRow>> {
    let mut builder = QueryBuilder::new(
        "SELECT tx_hash, output_index, address, consumed_by_tx, datum FROM utxos WHERE ",
    );

    builder.push(column);
    builder.push(" IN (");
    {
        let mut separated = builder.separated(", ");
        for hash in hashes {
            separated.push_bind(hash);
        }
    }
    builder.push(")");

    let query = builder.build_query_as::<UtxoRow>();
    let rows = query.fetch_all(pool).await?;

    Ok(rows)
}

async fn fetch_amounts(pool: &SqlitePool, utxos: &[UtxoRow]) -> Result<Vec<AmountRow>> {
    if utxos.is_empty() {
        return Ok(vec![]);
    }

    let mut tx_hashes = HashSet::new();
    let mut keys = HashSet::new();
    for utxo in utxos {
        tx_hashes.insert(utxo.tx_hash.clone());
        keys.insert((utxo.tx_hash.clone(), utxo.output_index));
    }

    let mut builder = QueryBuilder::new(
        "SELECT tx_hash, output_index, unit, quantity FROM utxo_amount WHERE tx_hash IN (",
    );
    {
        let mut separated = builder.separated(", ");
        for tx_hash in &tx_hashes {
            separated.push_bind(tx_hash);
        }
    }
    builder.push(")");

    let query = builder.build_query_as::<AmountRow>();
    let rows = query.fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .filter(|row| keys.contains(&(row.tx_hash.clone(), row.output_index)))
        .collect())
}

fn build_responses(
    rows: Vec<TxRow>,
    utxos: Vec<UtxoRow>,
    amounts: Vec<AmountRow>,
) -> Result<Vec<TxResponse>> {
    let mut amount_map: HashMap<(String, i64), Vec<Amount>> = HashMap::new();
    for row in amounts {
        let key = (row.tx_hash, row.output_index);
        amount_map.entry(key).or_default().push(Amount {
            unit: row.unit,
            quantity: row.quantity,
        });
    }

    let hash_set: HashSet<String> = rows.iter().map(|row| row.hash.clone()).collect();
    let mut inputs_map: HashMap<String, Vec<Utxo>> = HashMap::new();
    let mut outputs_map: HashMap<String, Vec<Utxo>> = HashMap::new();

    for row in utxos {
        let output_index = u32::try_from(row.output_index)
            .context("Invalid output_index value in database")?;
        let key = (row.tx_hash.clone(), row.output_index);
        let utxo = Utxo {
            address: row.address,
            tx_hash: row.tx_hash.clone(),
            output_index,
            amount: amount_map.get(&key).cloned().unwrap_or_default(),
            datum: row.datum,
            consumed_by_tx: row.consumed_by_tx.clone(),
        };

        if hash_set.contains(&row.tx_hash) {
            outputs_map.entry(row.tx_hash.clone()).or_default().push(utxo.clone());
        }

        if let Some(consumed_by_tx) = row.consumed_by_tx {
            if hash_set.contains(&consumed_by_tx) {
                inputs_map.entry(consumed_by_tx).or_default().push(utxo);
            }
        }
    }

    let responses = rows
        .into_iter()
        .map(|row| TxResponse {
            hash: row.hash.clone(),
            tx_name: row.tx_name,
            inputs: inputs_map.remove(&row.hash).unwrap_or_default(),
            outputs: outputs_map.remove(&row.hash).unwrap_or_default(),
        })
        .collect();

    Ok(responses)
}
