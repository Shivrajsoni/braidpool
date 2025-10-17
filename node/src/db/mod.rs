pub mod db_handlers;
pub mod init_db;

///These are the types representing the tuple
/// values for each of the relation defined under schema.sql which will be used
/// while appending to each table
#[derive(Debug, Clone,sqlx::FromRow)]
pub struct BeadTuple {
    pub id: Option<i64>,
    pub hash: String,
    pub nVersion: String,
    pub hashPrevBlock: String,
    pub hashMerkleRoot: String,
    pub nTime: String,
    pub nBits: String,
    pub nNonce: String,
    pub payout_address: String,
    pub start_timestamp: i64,
    pub comm_pub_key: String,
    pub min_target: String,
    pub weak_target: String,
    pub miner_ip: String,
    pub extra_nonce: String,
    pub broadcast_timestamp: i64,
    pub signature: String,
}

#[derive(Debug, Clone,sqlx::FromRow)]
pub struct TransactionTuple {
    pub bead_id: i64,
    pub txid: String,
}

#[derive(Debug, Clone,sqlx::FromRow)]
pub struct CohortIdTuple {
    pub id: i64,
}

#[derive(Debug, Clone,sqlx::FromRow)]
pub struct CohortTuple {
    pub bead_id: i64,
    pub cohort_id: Option<i64>,
}

#[derive(Debug, Clone,sqlx::FromRow)]
pub struct RelativeTuple {
    pub parent: i64,
    pub child: i64,
}

#[derive(Debug, Clone,sqlx::FromRow)]
pub struct ParentTimestampTuple {
    pub parent: i64,
    pub child: i64,
    pub timestamp: i64,
}

#[derive(Debug, Clone,sqlx::FromRow)]
pub struct AncestorTimestampTuple {
    pub bead_id: i64,
    pub ancestor: i64,
    pub timestamp: i64,
}
#[derive(Debug, Clone)]

pub enum InsertTupleTypes {
    BeadTuple {
        bead_tuple: BeadTuple,
    },
    TransactionTuple {
        transaction_tuple: TransactionTuple,
    },
    CohortIdTuple {
        cohort_id_tuple: CohortIdTuple,
    },
    CohortTuple {
        cohort_tuple: CohortTuple,
    },
    RelativeTuple {
        relative_tuple: RelativeTuple,
    },
    ParentTimestampTuple {
        parent_timestamp_tuple: ParentTimestampTuple,
    },
    AncestorTimestampTuple {
        ancestor_timestamp_tuple: AncestorTimestampTuple,
    },
}
#[derive(Debug, Clone)]
pub enum BraidpoolDBTypes {
    InsertTupleTypes { query: InsertTupleTypes },
}
