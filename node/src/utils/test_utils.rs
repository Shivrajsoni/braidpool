#[cfg(test)]
use super::BeadHash;
#[cfg(test)]
use crate::bead::Bead;
#[cfg(test)]
use crate::committed_metadata::CommittedMetadata;
#[cfg(test)]
pub use crate::committed_metadata::TimeVec;
#[cfg(test)]
use crate::uncommitted_metadata::UnCommittedMetadata;
#[cfg(test)]
pub use bitcoin::ecdsa::Signature;
#[cfg(test)]
use bitcoin::BlockHeader;
#[cfg(test)]
pub use bitcoin::{absolute::Time, p2p::address::AddrV2, PublicKey, Transaction};
#[cfg(test)]
pub mod test_utility_functions {
    use std::{collections::HashSet, str::FromStr};

    use bitcoin::{
        pow::CompactTargetExt, BlockHash, BlockTime, BlockVersion, CompactTarget, EcdsaSighashType,
        TxMerkleNode,
    };
    use rand::{rngs::OsRng, thread_rng, RngCore};
    use secp256k1::{Message, Secp256k1, SecretKey};

    pub use super::*;
    pub struct TestUnCommittedMetadataBuilder {
        extra_nonce: i32,
        broadcast_timestamp: Option<Time>,
        signature: Option<Signature>,
    }

    #[cfg(test)]
    impl TestUnCommittedMetadataBuilder {
        pub fn new() -> Self {
            Self {
                extra_nonce: 0,
                broadcast_timestamp: None,
                signature: None,
            }
        }

        pub fn extra_nonce(mut self, nonce: i32) -> Self {
            self.extra_nonce = nonce;
            self
        }

        pub fn broadcast_timestamp(mut self, time: Time) -> Self {
            self.broadcast_timestamp = Some(time);
            self
        }

        pub fn signature(mut self, sig: Signature) -> Self {
            self.signature = Some(sig);
            self
        }

        pub fn build(self) -> UnCommittedMetadata {
            UnCommittedMetadata {
                extra_nonce: self.extra_nonce,
                broadcast_timestamp: self
                    .broadcast_timestamp
                    .expect("broadcast_timestamp is required"),
                signature: self.signature.expect("signature is required"),
            }
        }
    }
    #[cfg(test)]
    pub struct TestCommittedMetadataBuilder {
        transactions: Vec<Transaction>,
        parents: std::collections::HashSet<BeadHash>,
        parent_bead_timestamps: Option<TimeVec>,
        payout_address: Option<String>,
        start_timestamp: Option<Time>,
        comm_pub_key: Option<PublicKey>,
        min_target: Option<CompactTarget>,
        weak_target: Option<CompactTarget>,
        miner_ip: Option<String>,
    }

    #[cfg(test)]
    impl TestCommittedMetadataBuilder {
        pub fn new() -> Self {
            Self {
                transactions: Vec::new(),
                parents: HashSet::new(),
                parent_bead_timestamps: None,
                payout_address: None,
                start_timestamp: None,
                comm_pub_key: None,
                min_target: None,
                weak_target: None,
                miner_ip: None,
            }
        }

        pub fn transactions(mut self, txs: Vec<Transaction>) -> Self {
            self.transactions = txs;
            self
        }

        pub fn parents(mut self, parents: HashSet<BeadHash>) -> Self {
            self.parents = parents;
            self
        }

        pub fn parent_bead_timestamps(mut self, times: TimeVec) -> Self {
            self.parent_bead_timestamps = Some(times);
            self
        }

        pub fn payout_address(mut self, address: String) -> Self {
            self.payout_address = Some(address);
            self
        }

        pub fn start_timestamp(mut self, time: Time) -> Self {
            self.start_timestamp = Some(time);
            self
        }

        pub fn comm_pub_key(mut self, key: PublicKey) -> Self {
            self.comm_pub_key = Some(key);
            self
        }

        pub fn miner_ip(mut self, ip: String) -> Self {
            self.miner_ip = Some(ip);
            self
        }
        pub fn min_target(mut self, min_target: CompactTarget) -> Self {
            self.min_target = Some(min_target);
            self
        }
        pub fn weak_target(mut self, weak_target: CompactTarget) -> Self {
            self.weak_target = Some(weak_target);
            self
        }
        pub fn build(self) -> CommittedMetadata {
            CommittedMetadata {
                transactions: self.transactions,
                parents: self.parents,
                parent_bead_timestamps: self
                    .parent_bead_timestamps
                    .expect("parent_bead_timestamps is required"),
                payout_address: self.payout_address.expect("payout_address is required"),
                start_timestamp: self
                    .start_timestamp
                    .expect("observed_time_at_node is required"),
                comm_pub_key: self.comm_pub_key.expect("comm_pub_key is required"),
                min_target: self.min_target.expect("min_target is required"),
                weak_target: self.weak_target.expect("weak_target is required"),
                miner_ip: self.miner_ip.expect("miner_ip is required"),
            }
        }
    }
    #[cfg(test)]
    pub struct TestBeadBuilder {
        block_header: Option<BlockHeader>,
        committed_metadata: Option<CommittedMetadata>,
        uncommitted_metadata: Option<UnCommittedMetadata>,
    }

    #[cfg(test)]
    impl TestBeadBuilder {
        pub fn new() -> Self {
            Self {
                block_header: None,
                committed_metadata: None,
                uncommitted_metadata: None,
            }
        }

        pub fn block_header(mut self, block_header: BlockHeader) -> Self {
            self.block_header = Some(block_header);
            self
        }

        pub fn committed_metadata(mut self, committed_metadata: CommittedMetadata) -> Self {
            self.committed_metadata = Some(committed_metadata);
            self
        }

        pub fn uncommitted_metadata(mut self, uncommitted_metadata: UnCommittedMetadata) -> Self {
            self.uncommitted_metadata = Some(uncommitted_metadata);
            self
        }

        pub fn build(self) -> Bead {
            Bead {
                block_header: self.block_header.expect("BlockHeader is required"),
                committed_metadata: self
                    .committed_metadata
                    .expect("CommittedMetadata is required"),
                uncommitted_metadata: self
                    .uncommitted_metadata
                    .expect("UnCommittedMetadata is required"),
            }
        }
    }
    fn generate_random_public_key_string() -> String {
        let secp = Secp256k1::new();
        let mut rng = thread_rng();
        let secret_key = SecretKey::new(&mut rng);
        let public_key = PublicKey::new(secret_key.public_key(&secp));
        public_key.to_string()
    }

    pub fn emit_bead() -> Bead {
        // This function creates a random bead for testing purposes.

        let random_public_key = generate_random_public_key_string()
            .parse::<bitcoin::PublicKey>()
            .unwrap();
        // Generate a reasonable timestamp (between 2020-01-01 and now)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        let current_time = bitcoin::absolute::Time::from_consensus(now).unwrap();

        let _address = String::from("127.0.0.1:8888");
        let public_key = random_public_key;
        let socket: String = String::from("127.0.0.1");
        let time_hash_set = TimeVec(Vec::new());
        let parent_hash_set: HashSet<BlockHash> = HashSet::new();
        let weak_target = CompactTarget::from_unprefixed_hex("1d00ffff").unwrap();
        let min_target = CompactTarget::from_unprefixed_hex("1d00ffff").unwrap();
        let time_val = current_time;

        let committed_metadata = TestCommittedMetadataBuilder::new()
            .comm_pub_key(public_key)
            .miner_ip(socket)
            .start_timestamp(time_val)
            .parents(parent_hash_set)
            .parent_bead_timestamps(time_hash_set)
            .payout_address(_address)
            .min_target(min_target)
            .weak_target(weak_target)
            .transactions(vec![])
            .build();

        let extra_nonce = rand::random::<i32>();
        let secp = Secp256k1::new();

        // Generate random secret key
        let mut rng = OsRng::default();
        let (secret_key, _) = secp.generate_keypair(&mut rng);

        // Create random 32-byte message
        let mut msg_bytes = [0u8; 32];
        rng.fill_bytes(&mut msg_bytes);
        let msg = Message::from_digest(msg_bytes);

        // Sign the message
        let signature = secp.sign_ecdsa(&msg, &secret_key);

        // DER encode the signature and get hex
        let der_sig = signature.serialize_der();
        let hex = hex::encode(der_sig);

        let sig = Signature {
            signature: secp256k1::ecdsa::Signature::from_str(&hex).unwrap(),
            sighash_type: EcdsaSighashType::All,
        };

        let uncommitted_metadata = TestUnCommittedMetadataBuilder::new()
            .broadcast_timestamp(time_val)
            .extra_nonce(extra_nonce)
            .signature(sig)
            .build();
        let bytes: [u8; 32] = [0u8; 32];

        let test_block_header = BlockHeader {
            version: BlockVersion::TWO,
            prev_blockhash: BlockHash::from_byte_array(bytes),
            bits: CompactTarget::from_consensus(486604799),
            nonce: rand::random::<u32>(),
            time: BlockTime::from_u32(rand::random::<u32>()),
            merkle_root: TxMerkleNode::from_byte_array(bytes),
        };

        let test_bead = TestBeadBuilder::new()
            .block_header(test_block_header)
            .committed_metadata(committed_metadata)
            .uncommitted_metadata(uncommitted_metadata)
            .build();
        test_bead
    }
}
