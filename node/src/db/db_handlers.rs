use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    bead::Bead,
    braid::{consensus_functions, Braid},
    db::{
        init_db::init_db, AncestorTimestampTuple, BeadTuple, BraidpoolDBTypes, CohortIdTuple,
        CohortTuple, InsertTupleTypes, ParentTimestampTuple, RelativeTuple, TransactionTuple,
    },
    error::DBErrors,
};
use futures::lock::Mutex;
use num::ToPrimitive;
use sqlx::{Pool, Sqlite};
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
    pub async fn insert_sequential_insert_bead(&self, bead: Bead) -> Result<(), DBErrors> {
        let mut braid_parent_set: HashMap<usize, HashSet<usize>> = HashMap::new();
        let braid_data = self.local_braid_arc.read().await;
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
        let braid_children_set: HashMap<usize, HashSet<usize>> =
            consensus_functions::reverse(&braid_data, &braid_parent_set);
        //Constructing ancestor set
        let mut ancestor_mapping: HashMap<usize, HashSet<usize>> = HashMap::new();
        consensus_functions::updating_ancestors(
            &braid_data,
            bead.block_header.block_hash(),
            &mut ancestor_mapping,
            &braid_parent_set,
        );
        //Begin the insertion transaction
        let mut tx = self.db_connection.lock().await.begin().await.unwrap();
        let bead_id = match sqlx::query(
            r#"
            INSERT INTO bead (
                hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime, 
                nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
                min_target, weak_target, miner_ip, extra_nonce, 
                broadcast_timestamp, signature
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(bead.block_header.block_hash().to_string())
        .bind(bead.block_header.version.to_consensus().to_string())
        .bind(bead.block_header.prev_blockhash.to_string())
        .bind(bead.block_header.merkle_root.to_string())
        .bind(bead.block_header.time.to_u32().to_string())
        .bind(bead.block_header.bits.to_hex())
        .bind(bead.block_header.nonce.to_string())
        .bind(bead.committed_metadata.payout_address)
        .bind(bead.committed_metadata.start_timestamp.to_string())
        .bind(bead.committed_metadata.comm_pub_key.to_string())
        .bind(bead.committed_metadata.min_target.to_hex())
        .bind(bead.committed_metadata.weak_target.to_hex())
        .bind(bead.committed_metadata.miner_ip)
        .bind(bead.uncommitted_metadata.extra_nonce.to_string())
        .bind(bead.uncommitted_metadata.broadcast_timestamp.to_string())
        .bind(bead.uncommitted_metadata.signature.to_string())
        .execute(&mut *tx)
        .await
        {
            Ok(value) => value.last_insert_rowid(),
            Err(error) => {
                tx.rollback().await.unwrap();
                return Err(DBErrors::TupleNotInserted {
                    error: error.to_string(),
                });
            }
        };
        //Considering the index of the beads in braid will be same as the (insertion ids-1)
        let current_bead_parent_set = braid_parent_set.get(&(bead_id as usize)).unwrap();
        let current_bead_child_set = braid_children_set.get(&(bead_id as usize)).unwrap();
        let mut relative_tuples: Vec<RelativeTuple> = Vec::new();
        let mut parent_timestamp_tuples: Vec<ParentTimestampTuple> = Vec::new();
        //Constructing relatives and parent_timestamps
        for parent_bead in current_bead_parent_set {
            relative_tuples.push(RelativeTuple {
                parent: (*parent_bead as i64) + 1,
                child: bead_id,
            });
            let current_parent_timestamp = braid_data
                .beads
                .get(*parent_bead)
                .unwrap()
                .committed_metadata
                .start_timestamp;
            parent_timestamp_tuples.push(ParentTimestampTuple {
                parent: (*parent_bead as i64) + 1,
                child: bead_id,
                timestamp: current_parent_timestamp.to_u32().to_i64().unwrap(),
            });
        }
        for child_bead in current_bead_child_set {
            relative_tuples.push(RelativeTuple {
                parent: bead_id,
                child: (*child_bead as i64) + 1,
            });
            parent_timestamp_tuples.push(ParentTimestampTuple {
                parent: bead_id,
                child: (*child_bead as i64) + 1,
                timestamp: bead
                    .committed_metadata
                    .start_timestamp
                    .to_u32()
                    .to_i64()
                    .unwrap(),
            });
        }
        //Inserting relatives in batch
        let mut relative_query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
            sqlx::QueryBuilder::new("INSERT INTO Relatives (child, parent) ");

        relative_query_builder.push_values(relative_tuples.iter(), |mut b, rel_tuple| {
            b.push_bind(rel_tuple.child).push_bind(rel_tuple.parent);
        });

        if let Err(error) = relative_query_builder.build().execute(&mut *tx).await {
            tx.rollback().await.unwrap();
            return Err(DBErrors::TupleNotInserted {
                error: format!("Relative batch insert failed: {}", error),
            });
        }
        let mut transaction_tuples: Vec<TransactionTuple> = Vec::new();
        for bead_tx in bead.committed_metadata.transactions.iter() {
            transaction_tuples.push(TransactionTuple {
                bead_id,
                txid: bead_tx.compute_txid().to_string(),
            });
        }
        //Building query builder for builk insertion operations
        let mut transaction_query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
            sqlx::QueryBuilder::new("INSERT INTO Transactions (bead_id, txid) ");
        transaction_query_builder.push_values(transaction_tuples.iter(), |mut b, tx_tuple| {
            b.push_bind(bead_id).push_bind(&tx_tuple.txid);
        });
        if let Err(error) = transaction_query_builder.build().execute(&mut *tx).await {
            tx.rollback().await.unwrap();
            return Err(DBErrors::TupleNotInserted {
                error: format!("Transaction batch insert failed: {}", error),
            });
        }
        let mut parent_timestamps_query_builder = sqlx::QueryBuilder::<Sqlite>::new(
            "INSERT INTO ParentTimestamps (parent, child, timestamp) ",
        );

        parent_timestamps_query_builder.push_values(
            parent_timestamp_tuples.iter(),
            |mut b, tuple| {
                b.push_bind(&tuple.parent)
                    .push_bind(&tuple.child)
                    .push_bind(&tuple.timestamp);
            },
        );

        if let Err(error) = parent_timestamps_query_builder
            .build()
            .execute(&mut *tx)
            .await
        {
            tx.rollback().await.unwrap();
            return Err(DBErrors::TupleNotInserted {
                error: error.to_string(),
            });
        }
        match tx.commit().await {
            Ok(_) => {
                log::info!("All related insertions committed successfully");
                Ok(())
            }
            Err(error) => Err(DBErrors::InsertionTransactionNotCommitted {
                error: error.to_string(),
                query_name: "Combined insert transaction".to_string(),
            }),
        }
    }
    //Individual insertion operations
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
                hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime, 
                nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
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
        match sqlx::query("INSERT INTO Transactions (bead_id, txid) VALUES (?, ?)")
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
            "INSERT INTO ParentTimestamps (parent, child, timestamp) VALUES (?, ?, ?)",
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
            "INSERT INTO AncestorTimestamps (bead_id, ancestor, timestamp) VALUES (?, ?, ?)",
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
        match sqlx::query("INSERT INTO CohortIds (id) VALUES (?)")
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
        match sqlx::query("INSERT INTO Cohorts (bead_id, cohort_id) VALUES (?, ?)")
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
        match sqlx::query("INSERT INTO Relatives (parent, child) VALUES (?, ?)")
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
                        let _res = self
                            .insert_ancestor_timestamp(ancestor_timestamp_tuple)
                            .await;
                    }
                    InsertTupleTypes::BeadTuple { bead_tuple } => {
                        let _res = self.insert_bead(bead_tuple).await;
                    }
                    InsertTupleTypes::CohortIdTuple { cohort_id_tuple } => {
                        let _res = self.insert_cohort_id(cohort_id_tuple).await;
                    }
                    InsertTupleTypes::CohortTuple { cohort_tuple } => {
                        let _res = self.insert_cohort_tuple(cohort_tuple).await;
                    }
                    InsertTupleTypes::ParentTimestampTuple {
                        parent_timestamp_tuple,
                    } => {
                        let _res = self.insert_parent_timestamp(parent_timestamp_tuple).await;
                    }
                    InsertTupleTypes::RelativeTuple { relative_tuple } => {
                        let _res = self.insert_relative_tuple(relative_tuple).await;
                    }
                    InsertTupleTypes::TransactionTuple { transaction_tuple } => {
                        let _res = self.insert_transaction(transaction_tuple).await;
                    }
                    InsertTupleTypes::InsertBeadSequentially { bead_to_insert } => {
                        let _res = self.insert_sequential_insert_bead(bead_to_insert).await;
                    }
                },
            }
        }
    }
}

//Fetch handlers globally accesible
pub async fn fetch_transactions_by_bead_id(
    db_connection_arc: Arc<Mutex<Pool<Sqlite>>>,
    bead_id: i64,
) -> Result<Vec<TransactionTuple>, DBErrors> {
    let rows = match sqlx::query_as::<_, TransactionTuple>(
        "SELECT  txid,bead_id FROM Transactions WHERE bead_id = ?",
    )
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
) -> Result<Option<BeadTuple>, DBErrors> {
    let row = match sqlx::query_as::<_, BeadTuple>("SELECT * FROM bead WHERE hash = ?")
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

    Ok(row)
}
pub async fn fetch_beads_by_cohort_id(
    db_connection_arc: Arc<Mutex<Pool<Sqlite>>>,
    cohort_id: i64,
) -> Result<Vec<BeadTuple>, DBErrors> {
    let rows = match sqlx::query_as::<_, BeadTuple>(
        r#"
        SELECT b.*
        FROM bead b
        JOIN Cohorts c ON c.bead_id = b.id
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
) -> Result<Vec<BeadTuple>, DBErrors> {
    let rows = match sqlx::query_as::<_, BeadTuple>(
        r#"
        SELECT b.*
        FROM bead b
        JOIN Relatives r ON r.child = b.id
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
    use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
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
        let mut insert_test_bead_tx = pool.begin().await.unwrap();

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
        let res = match sqlx::query(
            r#"
        INSERT INTO bead (
            hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime,
            nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
            min_target, weak_target, miner_ip, extra_nonce,
            broadcast_timestamp, signature
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&test_bead.hash)
        .bind(&test_bead.nVersion)
        .bind(&test_bead.hashPrevBlock)
        .bind(&test_bead.hashMerkleRoot)
        .bind(test_bead.nTime)
        .bind(test_bead.nBits)
        .bind(test_bead.nNonce)
        .bind(&test_bead.payout_address)
        .bind(test_bead.start_timestamp)
        .bind(&test_bead.comm_pub_key)
        .bind(test_bead.min_target)
        .bind(test_bead.weak_target)
        .bind(&test_bead.miner_ip)
        .bind(test_bead.extra_nonce)
        .bind(test_bead.broadcast_timestamp)
        .bind(&test_bead.signature)
        .execute(&mut *insert_test_bead_tx)
        .await
        {
            Ok(query_result) => {
                insert_test_bead_tx.commit().await.unwrap();
                query_result
            }
            Err(error) => {
                panic!("{:?}", error);
            }
        };
        let fetched_row = fetch_bead_by_bead_hash(
            Arc::new(Mutex::new(pool)),
            "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(
            "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".to_string(),
            fetched_row.unwrap_or(BeadTuple::default()).hash
        );

        assert_eq!(res.rows_affected(), 1);
    }
    #[tokio::test]
    async fn test_concurrent_access() {
        let pool = test_db_initializer().await;

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
        let insert_pool = pool.clone();
        let bead_clone_for_delete = test_bead.hash.clone();
        let insert_task_handle = tokio::task::spawn(async move {
            let mut tx = insert_pool.begin().await.unwrap();
            sqlx::query(
                r#"
            INSERT INTO bead (
                hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime,
                nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
                min_target, weak_target, miner_ip, extra_nonce,
                broadcast_timestamp, signature
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            )
            .bind(&test_bead.hash)
            .bind(&test_bead.nVersion)
            .bind(&test_bead.hashPrevBlock)
            .bind(&test_bead.hashMerkleRoot)
            .bind(&test_bead.nTime)
            .bind(&test_bead.nBits)
            .bind(&test_bead.nNonce)
            .bind(&test_bead.payout_address)
            .bind(test_bead.start_timestamp)
            .bind(&test_bead.comm_pub_key)
            .bind(&test_bead.min_target)
            .bind(&test_bead.weak_target)
            .bind(&test_bead.miner_ip)
            .bind(&test_bead.extra_nonce)
            .bind(test_bead.broadcast_timestamp)
            .bind(&test_bead.signature)
            .execute(&mut *tx)
            .await
            .unwrap();
            tx.commit().await.unwrap();
        });
        let delete_pool = pool.clone();
        let delete_task_handle = tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            let mut tx = delete_pool.begin().await.unwrap();
            sqlx::query("DELETE FROM bead WHERE hash = ?")
                .bind(bead_clone_for_delete)
                .execute(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        });

        let read_task_handle = tokio::task::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let row = fetch_bead_by_bead_hash(
                Arc::new(Mutex::new(pool)),
                "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".to_string(),
            )
            .await
            .unwrap();
            row
        });
        let (_, _, fetched_row) =
            tokio::join!(insert_task_handle, delete_task_handle, read_task_handle);
        assert_eq!(fetched_row.unwrap(), Option::None);
    }
    #[tokio::test]
    async fn test_insert_transaction() {
        let pool = test_db_initializer().await;
        let test_pool_arc = Arc::new(Mutex::new(pool.clone()));
        let test_transaction = TransactionTuple {
            bead_id: 1,
            txid: "9b8e7a6c5d4f3e2a1b0c9d8e7f6a5b4c3d2e1f0a9b8e7a6c5d4f3e2a1b0c9d8e".into(),
        };
        let mut insert_test_bead_tx = test_pool_arc.lock().await.begin().await.unwrap();

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
        let res = match sqlx::query(
            r#"
        INSERT INTO bead (
            hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime,
            nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
            min_target, weak_target, miner_ip, extra_nonce,
            broadcast_timestamp, signature
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&test_bead.hash)
        .bind(&test_bead.nVersion)
        .bind(&test_bead.hashPrevBlock)
        .bind(&test_bead.hashMerkleRoot)
        .bind(test_bead.nTime)
        .bind(test_bead.nBits)
        .bind(test_bead.nNonce)
        .bind(&test_bead.payout_address)
        .bind(test_bead.start_timestamp)
        .bind(&test_bead.comm_pub_key)
        .bind(test_bead.min_target)
        .bind(test_bead.weak_target)
        .bind(&test_bead.miner_ip)
        .bind(test_bead.extra_nonce)
        .bind(test_bead.broadcast_timestamp)
        .bind(&test_bead.signature)
        .execute(&mut *insert_test_bead_tx)
        .await
        {
            Ok(query_result) => {
                insert_test_bead_tx.commit().await.unwrap();
                query_result
            }
            Err(error) => {
                panic!("{:?}", error);
            }
        };
        assert_eq!(res.rows_affected(), 1);
        let fetched_row = fetch_bead_by_bead_hash(
            Arc::clone(&test_pool_arc),
            "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(
            fetched_row.unwrap().hash,
            "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".to_string()
        );
        let mut insert_test_transaction_tx = test_pool_arc.lock().await.begin().await.unwrap();

        let test_query_res =
            match sqlx::query("INSERT INTO Transactions (bead_id, txid) VALUES (?, ?)")
                .bind(test_transaction.bead_id)
                .bind(test_transaction.txid)
                .execute(&mut *insert_test_transaction_tx)
                .await
            {
                Ok(query_result) => {
                    let _res = insert_test_transaction_tx.commit().await;
                    println!("Test transaction insertion committed");
                    query_result
                }
                Err(error) => {
                    insert_test_transaction_tx.rollback().await.unwrap();
                    panic!("Rollback performed due to an error - {:?}.", error)
                }
            };
        assert_eq!(test_query_res.rows_affected(), 1);

        let fetch_test_tx_res = fetch_transactions_by_bead_id(Arc::clone(&test_pool_arc), 1)
            .await
            .unwrap();
        for r in fetch_test_tx_res {
            assert_eq!(
                r.txid,
                "9b8e7a6c5d4f3e2a1b0c9d8e7f6a5b4c3d2e1f0a9b8e7a6c5d4f3e2a1b0c9d8e".to_string()
            );
        }
    }
    #[tokio::test]
    async fn test_insert_cohort() {
        let pool = test_db_initializer().await;
        let pool_arc = Arc::new(Mutex::new(pool.clone()));

        let mut insert_test_bead_tx = pool_arc.lock().await.begin().await.unwrap();

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
        let res = match sqlx::query(
            r#"
        INSERT INTO bead (
            hash, nVersion, hashPrevBlock, hashMerkleRoot, nTime,
            nBits, nNonce, payout_address, start_timestamp, comm_pub_key,
            min_target, weak_target, miner_ip, extra_nonce,
            broadcast_timestamp, signature
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&test_bead.hash)
        .bind(&test_bead.nVersion)
        .bind(&test_bead.hashPrevBlock)
        .bind(&test_bead.hashMerkleRoot)
        .bind(test_bead.nTime)
        .bind(test_bead.nBits)
        .bind(test_bead.nNonce)
        .bind(&test_bead.payout_address)
        .bind(test_bead.start_timestamp)
        .bind(&test_bead.comm_pub_key)
        .bind(test_bead.min_target)
        .bind(test_bead.weak_target)
        .bind(&test_bead.miner_ip)
        .bind(test_bead.extra_nonce)
        .bind(test_bead.broadcast_timestamp)
        .bind(&test_bead.signature)
        .execute(&mut *insert_test_bead_tx)
        .await
        {
            Ok(query_result) => {
                insert_test_bead_tx.commit().await.unwrap();
                query_result
            }
            Err(error) => {
                panic!("{:?}", error);
            }
        };

        let bead_id = res.last_insert_rowid();
        assert_eq!(bead_id, 1);
        let test_cohort_id: CohortIdTuple = CohortIdTuple { id: 1 };
        let mut insert_test_cohortid_tx = pool_arc.lock().await.begin().await.unwrap();
        let test_cohort_id_insert_res = match sqlx::query("INSERT INTO CohortIds (id) VALUES (?)")
            .bind(test_cohort_id.id)
            .execute(&mut *insert_test_cohortid_tx)
            .await
        {
            Ok(query_result) => {
                insert_test_cohortid_tx.commit().await.unwrap();
                query_result
            }
            Err(error) => {
                insert_test_cohortid_tx.rollback().await.unwrap();
                panic!("{:?}", error)
            }
        };

        assert_eq!(test_cohort_id_insert_res.rows_affected(), 1);

        let mut insert_test_cohort_tx = pool_arc.lock().await.begin().await.unwrap();
        let test_cohort_tuple = CohortTuple {
            bead_id: 1,
            cohort_id: Some(1),
        };
        let cohort_insert_res =
            match sqlx::query("INSERT INTO Cohorts (bead_id, cohort_id) VALUES (?, ?)")
                .bind(test_cohort_tuple.bead_id)
                .bind(test_cohort_tuple.cohort_id)
                .execute(&mut *insert_test_cohort_tx)
                .await
            {
                Ok(query_result) => {
                    let _res = insert_test_cohort_tx.commit().await;
                    query_result
                }
                Err(error) => {
                    insert_test_cohort_tx.rollback().await.unwrap();
                    panic!("{:?}", error);
                }
            };

        assert_eq!(cohort_insert_res.rows_affected(), 1);

        let fetched_test_cohort_rows = fetch_beads_by_cohort_id(pool_arc.clone(), 1).await.unwrap();
        for bead in fetched_test_cohort_rows {
            assert_eq!(
                bead.hash,
                "3ce39ebba883260f6f9ac43865c077dcd99553434e4ad83fcdabea5b76255673".to_string()
            );
        }
    }
}
