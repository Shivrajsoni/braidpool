use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
};

use crate::{
    bead::Bead,
    braid::{consensus_functions, Braid},
    db::{init_db::init_db, BraidpoolDBTypes, InsertTupleTypes},
    error::DBErrors,
};
use bitcoin::{
    ecdsa::Signature, pow::CompactTargetExt, BlockHash, BlockTime, BlockVersion, CompactTarget,
    PublicKey, Transaction, TxMerkleNode,
};
use futures::lock::Mutex;
use num::ToPrimitive;
use sqlx::{Pool, Row, Sqlite};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    RwLock,
};

#[derive(Debug)]
pub struct DBHandler {
    //Query reciever inherit to handler only
    receiver: Receiver<BraidpoolDBTypes>,
    //Shared across tasks for accessing DB after contention using `Mutex`
    pub db_connection: Arc<Mutex<Pool<Sqlite>>>,
    local_braid_arc: Arc<RwLock<Braid>>,
}
// The DATABASE_URL environment variable must be set at build time to a database which it can prepare queries against; the database does not have to contain any data but must be the same kind (MySQL, Postgres, etc.) and have the same schema as the database you will be connecting to at runtime.
//A transaction starts with a call to Pool::begin or Connection::begin.
// A transaction should end with a call to commit or rollback. If neither are called before the transaction goes out-of-scope, rollback is called. In other words, rollback is called on drop if the transaction is still in-progress.
impl DBHandler {
    pub async fn new(
        local_braid_arc: Arc<RwLock<Braid>>,
    ) -> Result<(Self, Sender<BraidpoolDBTypes>), DBErrors> {
        log::info!("Initializing Schema for persistant DB");
        let connection = match init_db().await {
            Ok(conn) => conn,
            Err(error) => {
                log::error!("An error occurred while initializing and establishing connection with local DB {:?}.", error);
                return Err(DBErrors::ConnectionToDBNotEstablished {
                    error: error.to_string(),
                });
            }
        };
        let (db_handler_tx, db_handler_rx) = tokio::sync::mpsc::channel(1024);
        Ok((
            Self {
                receiver: db_handler_rx,
                db_connection: Arc::new(Mutex::new(connection)),
                local_braid_arc: local_braid_arc,
            },
            db_handler_tx,
        ))
    }
    //Insertion handlers private
    pub async fn insert_sequential_insert_bead(
        &self,
        bead: Bead,
        braid_parent_set: &HashMap<usize, HashSet<usize>>,
        _ancestor_mapping: &HashMap<usize, HashSet<usize>>,
        bead_id: &usize,
    ) -> Result<(), DBErrors> {
        log::info!("Sequential insertion query received from the query handler");
        let braid_data = self.local_braid_arc.read().await;
        let current_bead_parent_set = braid_parent_set.get(&(bead_id)).unwrap();
        log::info!(
            "PARENT SET  - {:?} for the bead id - {:?}",
            current_bead_parent_set,
            bead_id
        );
        let mut relative_tuples: Vec<(u64, u64)> = Vec::new();
        let mut parent_timestamp_tuples: Vec<(u64, u64, u64)> = Vec::new();
        let mut transaction_tuples: Vec<(u64, String)> = Vec::new();
        //Constructing relatives and parent_timestamps
        for parent_bead in current_bead_parent_set {
            relative_tuples.push(((*parent_bead as u64), (*bead_id as u64)));
            let current_parent_timestamp = braid_data
                .beads
                .get(*parent_bead)
                .unwrap()
                .committed_metadata
                .start_timestamp;
            parent_timestamp_tuples.push((
                (*parent_bead as u64),
                (*bead_id as u64),
                current_parent_timestamp.to_u32().to_u64().unwrap(),
            ));
        }
        for bead_tx in bead.committed_metadata.transactions.iter() {
            transaction_tuples.push(((*bead_id as u64), bead_tx.compute_txid().to_string()));
        }
        let transactions_values = transaction_tuples
            .iter()
            .map(|t| format!("(last_insert_rowid(), '{}')", t.1))
            .collect::<Vec<_>>()
            .join(", ");
        let parent_timestamps_values = parent_timestamp_tuples
            .iter()
            .map(|p| format!("({}, {}, {})", p.0, p.1, p.2))
            .collect::<Vec<_>>()
            .join(", ");

        let relatives_values = relative_tuples
            .iter()
            .map(|r| format!("({}, {})", r.1, r.0))
            .collect::<Vec<_>>()
            .join(", ");
        let sub_query_1 = "
        BEGIN TRANSACTION;
        INSERT INTO bead (
            id,hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime, 
            nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
            min_target, weak_target, miner_ip, extra_nonce, 
            broadcast_timestamp, signature
        ) VALUES (?,?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
";
        let mut sub_query_2 = String::new();
        let mut sub_query_3 = String::new();
        let mut sub_query_4 = String::new();

        if !transactions_values.is_empty() {
            sub_query_2 = format!(
                "INSERT INTO Transactions (bead_id, txid) VALUES {transactions_values};
                            "
            );
        }
        if !relatives_values.is_empty() {
            sub_query_3 =
                format!("INSERT INTO Relatives (child, parent) VALUES {relatives_values};");
        }
        if !parent_timestamps_values.is_empty() {
            sub_query_4 = format!(
                "INSERT INTO ParentTimestamps (parent, child, timestamp) VALUES {parent_timestamps_values};"
            );
        }

        let sql_query = format!(
            "{}{}{}{} COMMIT;",
            sub_query_1, sub_query_2, sub_query_3, sub_query_4
        );
        let hex_converted_version =
            hex::encode(bead.block_header.version.to_consensus().to_be_bytes());
        let hex_converted_nonce = hex::encode(bead.block_header.nonce.to_be_bytes());
        let hex_converted_ntime = hex::encode(bead.block_header.time.to_u32().to_be_bytes());
        #[allow(unused)]
        let hex_converted_extranonce =
            hex::encode(bead.uncommitted_metadata.extra_nonce.to_be_bytes());
        //All fields are in be format
        if let Err(e) = sqlx::query(&sql_query)
            .bind(*bead_id as i64)
            .bind(bead.block_header.block_hash().to_string())
            .bind(hex_converted_version)
            .bind(bead.block_header.prev_blockhash.to_string())
            .bind(bead.block_header.merkle_root.to_string())
            .bind(hex_converted_ntime)
            .bind(bead.block_header.bits.to_hex())
            .bind(hex_converted_nonce)
            .bind(bead.committed_metadata.payout_address)
            .bind(bead.committed_metadata.start_timestamp.to_string())
            .bind(bead.committed_metadata.comm_pub_key.to_string())
            .bind(bead.committed_metadata.min_target.to_hex())
            .bind(bead.committed_metadata.weak_target.to_hex())
            .bind(bead.committed_metadata.miner_ip)
            //TODO: Placeholder till splitting of `uncomitted_metadata` extranonce to extranonce1 + extranonce2
            .bind("0101010101010101".to_string())
            .bind(bead.uncommitted_metadata.broadcast_timestamp.to_string())
            .bind(bead.uncommitted_metadata.signature.to_string())
            .execute(&self.db_connection.lock().await.clone())
            .await
        {
            log::error!("Transaction failed to commit rolling back due to - {:?}", e);
            match sqlx::query("ROLLBACK;")
                .execute(&self.db_connection.lock().await.clone())
                .await
            {
                Ok(_query_res) => {
                    log::info!("Transaction rolled back successfully - {:?}", _query_res);
                }
                Err(error) => {
                    return Err(DBErrors::TransactionNotRolledBack {
                        error: error.to_string(),
                        query: "Combined insert transaction".to_string(),
                    })
                }
            };
            return Err(DBErrors::InsertionTransactionNotCommitted {
                error: e.to_string(),
                query_name: "Combined insert transaction".to_string(),
            });
        }
        log::info!("All related insertions committed successfully");
        Ok(())
    }
    //Individual insertion operations
    pub async fn insert_query_handler(&mut self) {
        log::info!("Query handler task started");
        while let Some(query_request) = self.receiver.recv().await {
            match query_request {
                BraidpoolDBTypes::InsertTupleTypes { query } => match query {
                    InsertTupleTypes::InsertBeadSequentially { bead_to_insert } => {
                        let braid_data = self.local_braid_arc.read().await;
                        let mut braid_parent_set: HashMap<usize, HashSet<usize>> = HashMap::new();
                        //Constructing the parent set
                        for bead in braid_data.beads.iter().enumerate() {
                            let parent_beads = &bead.1.committed_metadata.parents;
                            braid_parent_set.insert(bead.0, HashSet::new());
                            for parent_bead_hash in parent_beads.iter() {
                                let current_parent_bead_index = braid_data
                                    .bead_index_mapping
                                    .get(&*parent_bead_hash)
                                    .unwrap();
                                if let Some(value) = braid_parent_set.get_mut(&bead.0) {
                                    value.insert(*current_parent_bead_index);
                                }
                            }
                        }
                        //Constructing ancestor set, children set will be empty as it will become the next tip
                        let mut ancestor_mapping: HashMap<usize, HashSet<usize>> = HashMap::new();
                        consensus_functions::updating_ancestors(
                            &braid_data,
                            bead_to_insert.block_header.block_hash(),
                            &mut ancestor_mapping,
                            &braid_parent_set,
                        );
                        //Considering the index of the beads in braid will be same as the (insertion ids-1)

                        let bead_id = braid_data
                            .bead_index_mapping
                            .get(&bead_to_insert.block_header.block_hash())
                            .unwrap();
                        let _res = self
                            .insert_sequential_insert_bead(
                                bead_to_insert,
                                &braid_parent_set,
                                &ancestor_mapping,
                                bead_id,
                            )
                            .await;
                    }
                },
            }
        }
    }
}

pub async fn fetch_bead_by_bead_hash(
    db_connection_arc: Arc<Mutex<Pool<Sqlite>>>,
    bead_hash: BlockHash,
    local_braid_arc: Arc<RwLock<Braid>>,
) -> Result<Option<Bead>, DBErrors> {
    let mut bead_id = -1;
    let braid_data = local_braid_arc.read().await;
    let mut fetched_bead: Bead = Bead::default();
    match sqlx::query("SELECT * FROM bead WHERE hash = ?")
        .bind(bead_hash.to_string())
        .map(|row: sqlx::sqlite::SqliteRow| {
            let version = match i32::from_str_radix(row.get::<String, _>("nVersion").as_str(), 16) {
                Ok(v) => BlockVersion::from_consensus(v),
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "nVersion".to_string(),
                    })
                }
            };
            let prev = match BlockHash::from_str(&row.get::<String, _>("hashPrevBlock")) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "hashPrevBlock".to_string(),
                    })
                }
            };
            let merkle = match TxMerkleNode::from_str(&row.get::<String, _>("hashMerkleRoot")) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "hashMerkleRoot".to_string(),
                    })
                }
            };
            let time = match u32::from_str_radix(row.get::<String, _>("nTime").as_str(), 16) {
                Ok(v) => BlockTime::from_u32(v),
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "nTime".to_string(),
                    })
                }
            };
            let bits = match CompactTarget::from_unprefixed_hex(&row.get::<String, _>("nBits")) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "nBits".to_string(),
                    })
                }
            };
            let nonce = match u32::from_str_radix(row.get::<String, _>("nNonce").as_str(), 16) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "nNonce".to_string(),
                    })
                }
            };
            let comm_pub_key = match PublicKey::from_str(&row.get::<String, _>("comm_pub_key")) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "comm_pub_key".to_string(),
                    })
                }
            };
            let min_target =
                match CompactTarget::from_unprefixed_hex(&row.get::<String, _>("min_target")) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(DBErrors::TupleAtrributeParsingError {
                            error: e.to_string(),
                            attribute: "min_target".to_string(),
                        })
                    }
                };
            let weak_target =
                match CompactTarget::from_unprefixed_hex(&row.get::<String, _>("weak_target")) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(DBErrors::TupleAtrributeParsingError {
                            error: e.to_string(),
                            attribute: "weak_target".to_string(),
                        })
                    }
                };
            let extra_nonce = match i32::from_str_radix(&row.get::<String, _>("extra_nonce"), 16) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "extra_nonce".to_string(),
                    })
                }
            };
            let start_timestamp =
                match bitcoin::blockdata::locktime::absolute::MedianTimePast::from_u32(
                    row.get::<u32, _>("start_timestamp"),
                ) {
                    Ok(val) => val,
                    Err(error) => {
                        return Err(DBErrors::TupleAtrributeParsingError {
                            error: error.to_string(),
                            attribute: "startTimestamp".to_string(),
                        })
                    }
                };

            let miner_ip = row.get::<String, _>("miner_ip");
            let payout_address = row.get::<String, _>("payout_address");

            let signature = match Signature::from_str(row.get::<String, _>("signature").as_str()) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DBErrors::TupleAtrributeParsingError {
                        error: e.to_string(),
                        attribute: "signature".to_string(),
                    })
                }
            };

            let broadcast_timestamp =
                match bitcoin::blockdata::locktime::absolute::MedianTimePast::from_u32(
                    row.get::<u32, _>("broadcast_timestamp"),
                ) {
                    Ok(val) => val,
                    Err(error) => {
                        return Err(DBErrors::TupleAtrributeParsingError {
                            error: error.to_string(),
                            attribute: "broadcastTimestamp".to_string(),
                        })
                    }
                };

            bead_id = row.get::<i64, _>("id");
            fetched_bead.block_header.bits = bits;
            fetched_bead.block_header.merkle_root = merkle;
            fetched_bead.block_header.nonce = nonce;
            fetched_bead.block_header.prev_blockhash = prev;
            fetched_bead.block_header.time = time;
            fetched_bead.block_header.version = version;
            fetched_bead.uncommitted_metadata.signature = signature;
            fetched_bead.uncommitted_metadata.extra_nonce = extra_nonce;
            fetched_bead.uncommitted_metadata.broadcast_timestamp = broadcast_timestamp;
            fetched_bead.committed_metadata.comm_pub_key = comm_pub_key;
            fetched_bead.committed_metadata.min_target = min_target;
            fetched_bead.committed_metadata.miner_ip = miner_ip;
            fetched_bead.committed_metadata.payout_address = payout_address;
            fetched_bead.committed_metadata.start_timestamp = start_timestamp;
            fetched_bead.committed_metadata.weak_target = weak_target;

            Ok(())
        })
        .fetch_optional(&db_connection_arc.lock().await.clone())
        .await
    {
        Ok(_rows) => {
            println!("Bead with given bead hash fetched successfully");
        }
        Err(error) => {
            return Err(DBErrors::TupleNotFetched {
                error: error.to_string(),
            });
        }
    };
    let mut bead_txs: Vec<Transaction> = Vec::new();
    if bead_id != -1 {
        let rows = match sqlx::query("SELECT  txid,bead_id FROM Transactions WHERE bead_id = ?")
            .bind(bead_id)
            .fetch_all(&db_connection_arc.lock().await.clone())
            .await
        {
            Ok(rows) => rows,
            Err(error) => {
                return Err(DBErrors::TupleNotFetched {
                    error: error.to_string(),
                });
            }
        };
        for tx_row in rows {
            let _txid = tx_row.get::<String, _>("txid");
            let bead_id = tx_row.get::<u64, _>("bead_id");
            let indexed_bead = braid_data.beads.get((bead_id - 1) as usize).unwrap();
            bead_txs.extend(
                indexed_bead
                    .committed_metadata
                    .transactions
                    .clone()
                    .into_iter(),
            );
            fetched_bead
                .committed_metadata
                .parents
                .extend(indexed_bead.committed_metadata.parents.clone().into_iter());
            fetched_bead
                .committed_metadata
                .parent_bead_timestamps
                .0
                .extend(
                    indexed_bead
                        .committed_metadata
                        .parent_bead_timestamps
                        .clone()
                        .0
                        .into_iter(),
                );
            break;
        }
    } else {
        return Ok(None);
    }
    Ok(Some(fetched_bead))
}
#[cfg(test)]
#[allow(unused)]
pub mod test {
    use super::*;
    use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
    use std::{fs, str::FromStr};
    const TEST_DB_URL: &str = "sqlite::memory:";
    use crate::{braid, utils::test_utils::test_utility_functions::emit_bead};
    async fn insert_bead(
        test_conn: Pool<Sqlite>,
        test_braid: Arc<RwLock<braid::Braid>>,
    ) -> Result<BlockHash, DBErrors> {
        let test_bead = emit_bead();
        let mut test_braid_data = test_braid.write().await;
        let status = test_braid_data.extend(&test_bead);
        println!("Bead extension status - {:?}", status);
        let mut braid_parent_set: HashMap<usize, HashSet<usize>> = HashMap::new();
        //Constructing the parent set
        for bead in test_braid_data.beads.iter().enumerate() {
            let parent_beads = &bead.1.committed_metadata.parents;
            braid_parent_set.insert(bead.0, HashSet::new());
            for parent_bead_hash in parent_beads.iter() {
                let current_parent_bead_index = test_braid_data
                    .bead_index_mapping
                    .get(&*parent_bead_hash)
                    .unwrap();
                if let Some(value) = braid_parent_set.get_mut(&bead.0) {
                    value.insert(*current_parent_bead_index);
                }
            }
        }
        //Constructing ancestor set, children set will be empty as it will become the next tip
        let mut ancestor_mapping: HashMap<usize, HashSet<usize>> = HashMap::new();
        consensus_functions::updating_ancestors(
            &test_braid_data,
            test_bead.block_header.block_hash(),
            &mut ancestor_mapping,
            &braid_parent_set,
        );
        //Considering the index of the beads in braid will be same as the (insertion ids-1)
        let bead_id = test_braid_data
            .bead_index_mapping
            .get(&test_bead.block_header.block_hash())
            .unwrap();
        let current_bead_parent_set = braid_parent_set.get(&(bead_id)).unwrap();
        let mut relative_tuples: Vec<(u64, u64)> = Vec::new();
        let mut parent_timestamp_tuples: Vec<(u64, u64, u64)> = Vec::new();
        let mut transaction_tuples: Vec<(u64, String)> = Vec::new();
        //Constructing relatives and parent_timestamps
        for parent_bead in current_bead_parent_set {
            relative_tuples.push(((*parent_bead as u64), (*bead_id as u64)));
            let current_parent_timestamp = test_braid_data
                .beads
                .get(*parent_bead)
                .unwrap()
                .committed_metadata
                .start_timestamp;
            parent_timestamp_tuples.push((
                (*parent_bead as u64),
                (*bead_id as u64),
                current_parent_timestamp.to_u32().to_u64().unwrap(),
            ));
        }
        for bead_tx in test_bead.committed_metadata.transactions.iter() {
            transaction_tuples.push(((*bead_id as u64), bead_tx.compute_txid().to_string()));
        }
        let transactions_values = transaction_tuples
            .iter()
            .map(|t| format!("(last_insert_rowid(), '{}')", t.1))
            .collect::<Vec<_>>()
            .join(", ");
        let parent_timestamps_values = parent_timestamp_tuples
            .iter()
            .map(|p| format!("({}, {}, {})", p.0, p.1, p.2))
            .collect::<Vec<_>>()
            .join(", ");

        let relatives_values = relative_tuples
            .iter()
            .map(|r| format!("({}, {})", r.1, r.0))
            .collect::<Vec<_>>()
            .join(", ");

        let sub_query_1 = "
            BEGIN TRANSACTION;
            INSERT INTO bead (
                id,hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime, 
                nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
                min_target, weak_target, miner_ip, extra_nonce, 
                broadcast_timestamp, signature
            ) VALUES (?,?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
    ";
        let mut sub_query_2 = String::new();
        let mut sub_query_3 = String::new();
        let mut sub_query_4 = String::new();

        if !transactions_values.is_empty() {
            sub_query_2 = format!(
                "INSERT INTO Transactions (bead_id, txid) VALUES {transactions_values};
                        "
            );
        }
        if !relatives_values.is_empty() {
            sub_query_3 =
                format!("INSERT INTO Relatives (child, parent) VALUES {relatives_values};");
        }
        if !parent_timestamps_values.is_empty() {
            sub_query_4 = format!(
            "INSERT INTO ParentTimestamps (parent, child, timestamp) VALUES {parent_timestamps_values};"
        );
        }
        let sql_query = format!(
            "{}{}{}{} COMMIT;",
            sub_query_1, sub_query_2, sub_query_3, sub_query_4
        );
        let hex_converted_version =
            hex::encode(test_bead.block_header.version.to_consensus().to_be_bytes());
        let hex_converted_nonce = hex::encode(test_bead.block_header.nonce.to_be_bytes());
        let hex_converted_ntime = hex::encode(test_bead.block_header.time.to_u32().to_be_bytes());
        #[allow(unused)]
        let hex_converted_extranonce =
            hex::encode(test_bead.uncommitted_metadata.extra_nonce.to_be_bytes());
        //All fields are in be format
        if let Err(e) = sqlx::query(&sql_query)
            .bind(*bead_id as i64)
            .bind(test_bead.block_header.block_hash().to_string())
            .bind(hex_converted_version)
            .bind(test_bead.block_header.prev_blockhash.to_string())
            .bind(test_bead.block_header.merkle_root.to_string())
            .bind(hex_converted_ntime)
            .bind(test_bead.block_header.bits.to_hex())
            .bind(hex_converted_nonce)
            .bind(test_bead.committed_metadata.payout_address)
            .bind(test_bead.committed_metadata.start_timestamp.to_string())
            .bind(test_bead.committed_metadata.comm_pub_key.to_string())
            .bind(test_bead.committed_metadata.min_target.to_hex())
            .bind(test_bead.committed_metadata.weak_target.to_hex())
            .bind(test_bead.committed_metadata.miner_ip)
            //TODO: Placeholder till splitting of `uncomitted_metadata` extranonce to extranonce1 + extranonce2
            .bind("0000000000000000".to_string())
            .bind(
                test_bead
                    .uncommitted_metadata
                    .broadcast_timestamp
                    .to_string(),
            )
            .bind(test_bead.uncommitted_metadata.signature.to_string())
            .execute(&test_conn)
            .await
        {
            println!("Transaction failed to commit rolling back due to - {:?}", e);
            match sqlx::query("ROLLBACK;").execute(&test_conn).await {
                Ok(_query_res) => {
                    println!("Transaction rolled back successfully - {:?}", _query_res);
                }
                Err(error) => {
                    return Err(DBErrors::TransactionNotRolledBack {
                        error: error.to_string(),
                        query: "Combined insert transaction".to_string(),
                    })
                }
            };
            return Err(DBErrors::InsertionTransactionNotCommitted {
                error: e.to_string(),
                query_name: "Combined insert transaction".to_string(),
            });
        }
        Ok(test_bead.block_header.block_hash())
    }

    pub async fn test_db_initializer() -> Pool<Sqlite> {
        let test_pool_settings = SqliteConnectOptions::from_str(TEST_DB_URL)
            .unwrap()
            .foreign_keys(true)
            .with_regexp()
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let test_pool = SqlitePool::connect_with(test_pool_settings).await.unwrap();
        let schema_path = std::env::current_dir().unwrap().join("src/db/schema.sql");
        let schema_sql = fs::read_to_string(&schema_path).unwrap();

        let setup_result = sqlx::query(&schema_sql.as_str()).execute(&test_pool).await;

        match setup_result {
            Ok(_) => {
                println!("Test Schema setup success");
            }
            Err(error) => {
                panic!("{:?}", error);
            }
        }

        test_pool
    }
    #[tokio::test]
    async fn test_insert_bead() {
        let pool = test_db_initializer().await;
        let genesis_beads = Vec::from([]);
        // Initializing the braid object with read write lock
        //for supporting concurrent readers and single writer
        let test_braid: Arc<RwLock<braid::Braid>> =
            Arc::new(RwLock::new(braid::Braid::new(genesis_beads)));
        let res = insert_bead(pool.clone(), test_braid.clone()).await;
        let fetched_bead = fetch_bead_by_bead_hash(
            Arc::new(Mutex::new(pool)),
            res.clone().unwrap(),
            test_braid.clone(),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            fetched_bead.block_header.block_hash().to_string(),
            res.unwrap().to_string()
        );
    }
}
