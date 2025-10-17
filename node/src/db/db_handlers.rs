use std::sync::Arc;

use crate::{
    db::{
        init_db::init_db, AncestorTimestampTuple, BeadTuple, BraidpoolDBTypes, CohortIdTuple,
        CohortTuple, InsertTupleTypes, ParentTimestampTuple, RelativeTuple, TransactionTuple,
    },
    error::DBErrors,
};
use futures::lock::Mutex;
use sqlx::{sqlite::SqliteRow, Pool, Sqlite};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct DBHandler {
    //Query reciever inherit to handler only
    receiver: Receiver<BraidpoolDBTypes>,
    //Shared across tasks for accessing DB after contention using `Mutex`
    pub db_connection: Arc<Mutex<Pool<Sqlite>>>,
}
// The DATABASE_URL environment variable must be set at build time to a database which it can prepare queries against; the database does not have to contain any data but must be the same kind (MySQL, Postgres, etc.) and have the same schema as the database you will be connecting to at runtime.
//A transaction starts with a call to Pool::begin or Connection::begin.
// A transaction should end with a call to commit or rollback. If neither are called before the transaction goes out-of-scope, rollback is called. In other words, rollback is called on drop if the transaction is still in-progress.
impl DBHandler {
    pub async fn new() -> (Self, Sender<BraidpoolDBTypes>) {
        let connection = match init_db().await {
            Ok(conn) => conn,
            Err(error) => {
                log::error!("An error occurred while initializing and establishing connection with local DB {:?}.", error);
                panic!("")
            }
        };
        let (db_handler_tx, db_handler_rx) = tokio::sync::mpsc::channel(1024);
        (
            Self {
                receiver: db_handler_rx,
                db_connection: Arc::new(Mutex::new(connection)),
            },
            db_handler_tx,
        )
    }
    //Insertion handlers private
    async fn insert_bead(&self, bead_tuple: BeadTuple) -> Result<u64, DBErrors> {
        //Starting the transaction
        let mut insert_bead_tx = self
            .db_connection
            .lock()
            .await
            .clone()
            .begin()
            .await
            .unwrap();

        match sqlx::query(
            r#"
            INSERT INTO bead (
                hash, n_version, hash_prev_block, hash_merkle_root, n_time, 
                n_bits, n_nonce, payout_address, start_timestamp, comm_pub_key,
                min_target, weak_target, miner_ip, extra_nonce, 
                broadcast_timestamp, signature
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&bead_tuple.hash)
        .bind(&bead_tuple.nVersion)
        .bind(&bead_tuple.hashPrevBlock)
        .bind(&bead_tuple.hashMerkleRoot)
        .bind(bead_tuple.nTime)
        .bind(bead_tuple.nBits)
        .bind(bead_tuple.nNonce)
        .bind(&bead_tuple.payout_address)
        .bind(bead_tuple.start_timestamp)
        .bind(&bead_tuple.comm_pub_key)
        .bind(bead_tuple.min_target)
        .bind(bead_tuple.weak_target)
        .bind(&bead_tuple.miner_ip)
        .bind(bead_tuple.extra_nonce)
        .bind(bead_tuple.broadcast_timestamp)
        .bind(&bead_tuple.signature)
        .execute(&mut *insert_bead_tx)
        .await
        {
            Ok(query_result) => {
                match insert_bead_tx.commit().await {
                    Ok(_) => {
                        log::info!("Insert bead transaction committed");
                        return Ok(query_result.rows_affected());
                    }
                    Err(error) => {
                        return Err(DBErrors::InsertionTransactionNotCommitted {
                            error: error.to_string(),
                            query_name: "Insertion of bead".to_string(),
                        })
                    }
                };
            }
            Err(error) => {
                insert_bead_tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        };
    }
    async fn insert_transaction(
        &self,
        transaction_tuple: TransactionTuple,
    ) -> Result<u64, DBErrors> {
        let mut insert_transaction_tx = self.db_connection.lock().await.begin().await.unwrap();
        match sqlx::query("INSERT INTO transaction (bead_id, txid) VALUES (?, ?)")
            .bind(transaction_tuple.bead_id)
            .bind(&transaction_tuple.txid)
            .execute(&mut *insert_transaction_tx)
            .await
        {
            Ok(query_result) => {
                match insert_transaction_tx.commit().await {
                    Ok(_) => {
                        log::info!(
                            "Insert transaction into transaction table, transaction committed"
                        );
                        return Ok(query_result.rows_affected());
                    }
                    Err(error) => {
                        return Err(DBErrors::InsertionTransactionNotCommitted {
                            error: error.to_string(),
                            query_name: "Insertion of transaction".to_string(),
                        })
                    }
                };
            }
            Err(error) => {
                insert_transaction_tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        };
    }
    async fn insert_parent_timestamp(
        &self,
        parent_timestamp_tuple: ParentTimestampTuple,
    ) -> Result<u64, DBErrors> {
        let mut insert_parent_timestamp_tx = self.db_connection.lock().await.begin().await.unwrap();
        match sqlx::query(
            "INSERT INTO parent_timestamp (parent, child, timestamp) VALUES (?, ?, ?)",
        )
        .bind(parent_timestamp_tuple.parent)
        .bind(parent_timestamp_tuple.child)
        .bind(parent_timestamp_tuple.timestamp)
        .execute(&mut *insert_parent_timestamp_tx)
        .await
        {
            Ok(query_result) => {
                match insert_parent_timestamp_tx.commit().await {
                    Ok(_) => {
                        log::info!("Insert parent timestamp transaction committed");
                        return Ok(query_result.rows_affected());
                    }
                    Err(error) => {
                        return Err(DBErrors::InsertionTransactionNotCommitted {
                            error: error.to_string(),
                            query_name: "Insertion of parent timestamp".to_string(),
                        })
                    }
                };
            }
            Err(error) => {
                insert_parent_timestamp_tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        };
    }
    async fn insert_ancestor_timestamp(
        &self,
        ancestor_timestamp_tuple: AncestorTimestampTuple,
    ) -> Result<u64, DBErrors> {
        let mut insert_ancestor_timestamp_tx =
            self.db_connection.lock().await.begin().await.unwrap();
        match sqlx::query(
            "INSERT INTO ancestor_timestamp (bead_id, ancestor, timestamp) VALUES (?, ?, ?)",
        )
        .bind(ancestor_timestamp_tuple.bead_id)
        .bind(ancestor_timestamp_tuple.ancestor)
        .bind(ancestor_timestamp_tuple.timestamp)
        .execute(&mut *insert_ancestor_timestamp_tx)
        .await
        {
            Ok(query_result) => {
                match insert_ancestor_timestamp_tx.commit().await {
                    Ok(_) => {
                        log::info!("Insert ancestor timestamp transaction committed");
                        return Ok(query_result.rows_affected());
                    }
                    Err(error) => {
                        return Err(DBErrors::InsertionTransactionNotCommitted {
                            error: error.to_string(),
                            query_name: "Insertion of ancestor timestamp".to_string(),
                        })
                    }
                };
            }
            Err(error) => {
                insert_ancestor_timestamp_tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        };
    }
    async fn insert_cohort_id(&self, cohort_id_tuple: CohortIdTuple) -> Result<u64, DBErrors> {
        let mut insert_cohort_id_tx = self.db_connection.lock().await.begin().await.unwrap();
        match sqlx::query("INSERT INTO cohort_id (id) VALUES (?)")
            .bind(cohort_id_tuple.id)
            .execute(&mut *insert_cohort_id_tx)
            .await
        {
            Ok(query_result) => {
                match insert_cohort_id_tx.commit().await {
                    Ok(_) => {
                        log::info!("Insert cohort id transaction committed");
                        return Ok(query_result.rows_affected());
                    }
                    Err(error) => {
                        return Err(DBErrors::InsertionTransactionNotCommitted {
                            error: error.to_string(),
                            query_name: "Insertion of cohort Id".to_string(),
                        })
                    }
                };
            }
            Err(error) => {
                insert_cohort_id_tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        };
    }
    async fn insert_cohort_tuple(&self, cohort_tuple: CohortTuple) -> Result<u64, DBErrors> {
        let mut insert_cohort_tx = self.db_connection.lock().await.begin().await.unwrap();
        match sqlx::query("INSERT INTO cohort (bead_id, cohort_id) VALUES (?, ?)")
            .bind(cohort_tuple.bead_id)
            .bind(cohort_tuple.cohort_id)
            .execute(&mut *insert_cohort_tx)
            .await
        {
            Ok(query_result) => {
                match insert_cohort_tx.commit().await {
                    Ok(_) => {
                        log::info!("Insert cohort committed");
                        return Ok(query_result.rows_affected());
                    }
                    Err(error) => {
                        return Err(DBErrors::InsertionTransactionNotCommitted {
                            error: error.to_string(),
                            query_name: "Insertion of cohort".to_string(),
                        })
                    }
                };
            }
            Err(error) => {
                insert_cohort_tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        };
    }
    async fn insert_relative_tuple(&self, relative_tuple: RelativeTuple) -> Result<u64, DBErrors> {
        let mut insert_relative_tx = self.db_connection.lock().await.begin().await.unwrap();
        match sqlx::query("INSERT INTO relative (parent, child) VALUES (?, ?)")
            .bind(relative_tuple.parent)
            .bind(relative_tuple.child)
            .execute(&mut *insert_relative_tx)
            .await
        {
            Ok(query_result) => {
                match insert_relative_tx.commit().await {
                    Ok(_) => {
                        log::info!("Insert relative beads transaction committed");
                        return Ok(query_result.rows_affected());
                    }
                    Err(error) => {
                        return Err(DBErrors::InsertionTransactionNotCommitted {
                            error: error.to_string(),
                            query_name: "Insertion of relative beads".to_string(),
                        })
                    }
                };
            }
            Err(error) => {
                insert_relative_tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        }
    }
    pub async fn insert_query_handler(&mut self) {
        while let Some(query_request) = self.receiver.recv().await {
            match query_request {
                BraidpoolDBTypes::InsertTupleTypes { query } => match query {
                    InsertTupleTypes::AncestorTimestampTuple {
                        ancestor_timestamp_tuple,
                    } => {
                        let res = self
                            .insert_ancestor_timestamp(ancestor_timestamp_tuple)
                            .await;
                    }
                    InsertTupleTypes::BeadTuple { bead_tuple } => {
                        let res = self.insert_bead(bead_tuple).await;
                    }
                    InsertTupleTypes::CohortIdTuple { cohort_id_tuple } => {
                        let res = self.insert_cohort_id(cohort_id_tuple).await;
                    }
                    InsertTupleTypes::CohortTuple { cohort_tuple } => {
                        let res = self.insert_cohort_tuple(cohort_tuple).await;
                    }
                    InsertTupleTypes::ParentTimestampTuple {
                        parent_timestamp_tuple,
                    } => {
                        let res = self.insert_parent_timestamp(parent_timestamp_tuple).await;
                    }
                    InsertTupleTypes::RelativeTuple { relative_tuple } => {
                        let res = self.insert_relative_tuple(relative_tuple).await;
                    }
                    InsertTupleTypes::TransactionTuple { transaction_tuple } => {
                        let res = self.insert_transaction(transaction_tuple).await;
                    }
                },
                _ => {
                    log::error!("Unknown query kindly check again");
                }
            }
        }
    }
}

//Fetch handlers globally accesible
pub async fn fetch_transactions_by_bead_id(
    db_connection_arc: Arc<Mutex<Pool<Sqlite>>>,
    bead_id: i64,
) -> Result<Vec<SqliteRow>, DBErrors> {
    let rows = match sqlx::query("SELECT  txid FROM transaction WHERE bead_id = ?")
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
    Ok(rows)
}
pub async fn fetch_bead_by_bead_hash(
    db_connection_arc: Arc<Mutex<Pool<Sqlite>>>,
    bead_hash: String,
) -> Result<SqliteRow, DBErrors> {
    let row = match sqlx::query("SELECT * FROM bead WHERE hash = ?")
        .bind(bead_hash)
        .fetch_optional(&db_connection_arc.lock().await.clone())
        .await
    {
        Ok(rows) => rows,
        Err(error) => {
            return Err(DBErrors::TupleNotFetched {
                error: error.to_string(),
            });
        }
    };

    Ok(row.unwrap())
}
pub async fn fetch_beads_by_cohort_id(
    db_connection_arc: Arc<Mutex<Pool<Sqlite>>>,
    cohort_id: i64,
) -> Result<Vec<SqliteRow>, DBErrors> {
    let rows = match sqlx::query(
        r#"
        SELECT b.*
        FROM bead b
        JOIN cohort c ON c.bead_id = b.id
        WHERE c.cohort_id = ?
        ORDER BY b.id
        "#,
    )
    .bind(cohort_id)
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
    Ok(rows)
}
pub async fn fetch_children_by_bead_id(
    db_connection_arc: Arc<Mutex<Pool<Sqlite>>>,
    parent_bead_id: i64,
) -> Result<Vec<SqliteRow>, DBErrors> {
    let rows = match sqlx::query(
        r#"
        SELECT b.*
        FROM bead b
        JOIN relatives r ON r.child = b.id
        WHERE r.parent = ?
        ORDER BY b.id
        "#,
    )
    .bind(parent_bead_id)
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
    Ok(rows)
}

#[cfg(test)]
pub mod test {
    use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
    use std::{fs, str::FromStr};

    use super::*;
    const TEST_DB_URL: &str = "sqlite::memory:";

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
                println!("Schema setup success");
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
        let mut tx = pool.begin().await.unwrap();

        let test_bead = BeadTuple {
            id: None,
            hash: "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".into(),
            nVersion: "00000001".into(),
            hashPrevBlock: "29979d4a18745698f43f113c111417838cf0c7c31282571b5e341f70e5d8a763"
                .into(),
            hashMerkleRoot: "c3051efaaaaba08599f02479ae76f32836a2b73433d408207034ab78609cba1c"
                .into(),
            nTime: "5f5f1000".into(),
            nBits: "1a2b3c4d".into(),
            nNonce: "030223ff".into(),
            payout_address: "bcrt1qpa77defz30uavu8lxef98q95rae6m7t8au9vp7".into(),
            start_timestamp: 123456789,
            comm_pub_key: "020202020202020202020202020202020202020202020202020202020202020202".into(),
            min_target: "1d0fff00".into(),
            weak_target: "ffffffff".into(),
            miner_ip: "127.0.0.1".into(),
            extra_nonce: "ffffffffffffffff".into(),
            broadcast_timestamp: 123456790,
            signature: "3046022100839c1fbc5304de944f697c9f4b1d01d1faeba32d751c0f7acb21ac8a0f436a72022100e89bd46bb3a5a62adc679f659b7ce876d83ee297c7a5587b2011c4fcc72eab45".into(),
        };

        let res = sqlx::query(
            r#"
        INSERT INTO bead (
            hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime, 
            nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
            min_target, weak_target, miner_ip, extra_nonce, 
            broadcast_timestamp, signature
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(test_bead.hash)
        .bind(test_bead.nVersion)
        .bind(test_bead.hashPrevBlock)
        .bind(test_bead.hashMerkleRoot)
        .bind(test_bead.nTime)
        .bind(test_bead.nBits)
        .bind(test_bead.nNonce)
        .bind(test_bead.payout_address)
        .bind(test_bead.start_timestamp)
        .bind(test_bead.comm_pub_key)
        .bind(test_bead.min_target)
        .bind(test_bead.weak_target)
        .bind(test_bead.miner_ip)
        .bind(test_bead.extra_nonce)
        .bind(test_bead.broadcast_timestamp)
        .bind(test_bead.signature)
        .execute(&mut *tx)
        .await;
        tx.commit().await.unwrap();
        let fetched_rows = sqlx::query_as::<_, BeadTuple>("select*from bead")
            .fetch_all(&pool)
            .await
            .unwrap();
        for row in fetched_rows.iter() {
            assert_eq!(
                "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".to_string(),
                row.hash
            );
        }
        assert_eq!(res.unwrap().rows_affected(), 1);
    }
}
