#![allow(unused)]
use crate::bead::Bead;
use crate::braid::AddBeadStatus;
use crate::braid_manager::BraidCommand;
use crate::cli::Cli;
use crate::committed_metadata::CommittedMetadata;
use crate::committed_metadata::TimeVec;
use crate::uncommitted_metadata::UnCommittedMetadata;
use crate::utils::BeadHash;
use bitcoin::absolute::Encodable;
use bitcoin::absolute::Time;
use bitcoin::ecdsa::Signature;
use bitcoin::p2p::ServiceFlags;
use bitcoin::{
    Address, BlockHash, BlockHeader, BlockTime, BlockVersion, CompactTarget, EcdsaSighashType,
    TxMerkleNode,
};
use core::panic;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::middleware::{Batch, Notification, Request, RpcServiceT};
use jsonrpsee::core::params::ArrayParams;
use jsonrpsee::core::traits::ToRpcParams;
use jsonrpsee::core::{async_trait, SubscriptionResult};
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::rpc_params;
use jsonrpsee::server::HttpBody;
use jsonrpsee::server::HttpResponse;
use jsonrpsee::server::PendingSubscriptionSink;
use jsonrpsee::types::{ErrorObject, ErrorObjectOwned};
use jsonrpsee::ws_client::WsClientBuilder;
use jsonrpsee::{ConnectionId, Extensions};
use log;
use node::bead;
use serde_json;
use serde_json::Value;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio::time;

//parsing the inital rpc command line all
pub async fn parse_arguments(
    rpc_arguments: Cli,
    server_addr: SocketAddr,
    rpc_handler_sender: Sender<String>,
) -> (String, String) {
    //parsing the cli command/method
    //constructing a client/request
    //receving a response and moving forward
    let cli_argument = rpc_arguments.rpc.unwrap();
    let cli_argument_ref = cli_argument.clone();
    let rpc_method = cli_argument_ref.get(0).unwrap();
    let rpc_method_arguments = &cli_argument.clone()[1..];

    // //initializing a client associated with the current node
    // //for receving the response from the server
    let target_uri = format!("http://{}", server_addr.to_string());
    let client_res: HttpClient = HttpClient::builder().build(target_uri).unwrap();

    match rpc_method.clone().as_str() {
        "getbead" => {
            if rpc_method_arguments.clone().len() != 1 {
                log::error!(
                    "Provide suitable arguments for method {}",
                    rpc_method.clone()
                );
                panic!();
            } else if rpc_method_arguments.clone().len() == 1 {
                let bead_hash = &rpc_method_arguments[0].clone();
                println!("{:?}", bead_hash);
                let mut method_params = ArrayParams::new();
                method_params.insert(bead_hash);
                let handler_tx_ref = rpc_handler_sender.clone();
                tokio::spawn(handle_request(
                    rpc_method.clone(),
                    method_params,
                    client_res,
                    handler_tx_ref,
                ));
            }
        }
        "getbeadcount" => {
            if !rpc_method_arguments.is_empty() {
                log::error!("Method {} does not take any arguments", rpc_method.clone());
                panic!();
            } else {
                let method_params = ArrayParams::new();
                let handler_tx_ref = rpc_handler_sender.clone();
                tokio::spawn(handle_request(
                    rpc_method.clone(),
                    method_params,
                    client_res,
                    handler_tx_ref,
                ));
            }
        }
        "getcohortcount" => {
            if !rpc_method_arguments.is_empty() {
                log::error!("Method {} does not take any arguments", rpc_method.clone());
                panic!();
            } else {
                let method_params = ArrayParams::new();
                let handler_tx_ref = rpc_handler_sender.clone();
                tokio::spawn(handle_request(
                    rpc_method.clone(),
                    method_params,
                    client_res,
                    handler_tx_ref,
                ));
            }
        }
        _ => {
            log::error!("Invalid rpc method");
            panic!();
        }
    }
    ("".to_string(), "".to_string())
}

//handling the request arising either from command line cli or from the external users
pub async fn handle_request(
    method: String,
    method_params: ArrayParams,
    client: HttpClient,
    rpc_handler_sender: Sender<String>,
) {
    let rpc_response: Result<String, jsonrpsee::core::ClientError> =
        client.request(&method, method_params.clone()).await;
    rpc_handler_sender.send(rpc_response.unwrap()).await;
}

//server side trait to be implemented for the handler
//that is the JSON-RPC handle to initiate the RPC context
//supporting both http and websockets
#[rpc(server)]
pub trait Rpc {
    //RPC methods supported by braid-API
    #[method(name = "getbead")]
    async fn get_bead(&self, bead_hash: String) -> Result<String, ErrorObjectOwned>;

    #[method(name = "addbead")]
    async fn add_bead(&self, bead_data: String) -> Result<String, ErrorObjectOwned>;

    #[method(name = "gettips")]
    async fn get_tips(&self) -> Result<String, ErrorObjectOwned>;

    #[method(name = "getbeadcount")]
    async fn get_bead_count(&self) -> Result<String, ErrorObjectOwned>;

    #[method(name = "getcohortcount")]
    async fn get_cohort_count(&self) -> Result<String, ErrorObjectOwned>;
}

// RPC Server implementation using channels
pub struct RpcServerImpl {
    braid_tx: mpsc::Sender<BraidCommand>,
}

impl RpcServerImpl {
    pub fn new(braid_tx: mpsc::Sender<BraidCommand>) -> Self {
        Self { braid_tx }
    }
}
#[async_trait]
impl RpcServer for RpcServerImpl {
    async fn get_bead(&self, bead_hash: String) -> Result<String, ErrorObjectOwned> {
        let hash = bead_hash
            .parse::<BeadHash>()
            .map_err(|_| ErrorObjectOwned::owned(1, "Invalid bead hash format", None::<()>))?;

        let (tx, rx) = oneshot::channel();

        self.braid_tx
            .send(BraidCommand::GetBead {
                hash,
                respond_to: tx,
            })
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;

        match rx
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?
        {
            Some(bead) => {
                let json = serde_json::to_string(&bead)
                    .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;
                Ok(json)
            }
            None => Err(ErrorObjectOwned::owned(3, "Bead not found", None::<()>)),
        }
    }

    async fn add_bead(&self, bead_data: String) -> Result<String, ErrorObjectOwned> {
        let bead: Bead = serde_json::from_str(&bead_data).map_err(|e| {
            ErrorObjectOwned::owned(1, format!("Invalid bead data: {}", e), None::<()>)
        })?;

        let (tx, rx) = oneshot::channel();

        self.braid_tx
            .send(BraidCommand::AddBead {
                bead,
                respond_to: tx,
            })
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;

        let status = rx
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;

        match status {
            AddBeadStatus::BeadAdded => Ok("Bead added successfully".to_string()),
            AddBeadStatus::DagAlreadyContainsBead => Ok("Bead already exists".to_string()),
            AddBeadStatus::InvalidBead => {
                Err(ErrorObjectOwned::owned(4, "Invalid bead", None::<()>))
            }
            AddBeadStatus::ParentsNotYetReceived => {
                Ok("Bead queued, waiting for parents".to_string())
            }
        }
    }

    async fn get_tips(&self) -> Result<String, ErrorObjectOwned> {
        let (tx, rx) = oneshot::channel();

        self.braid_tx
            .send(BraidCommand::GetTips { respond_to: tx })
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;

        let tips = rx
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;
        let tips_str: Vec<String> = tips.iter().map(|h| h.to_string()).collect();

        serde_json::to_string(&tips_str)
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))
    }

    async fn get_bead_count(&self) -> Result<String, ErrorObjectOwned> {
        let (tx, rx) = oneshot::channel();

        self.braid_tx
            .send(BraidCommand::GetBeadCount { respond_to: tx })
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;

        let count = rx
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;
        Ok(count.to_string())
    }

    async fn get_cohort_count(&self) -> Result<String, ErrorObjectOwned> {
        let (tx, rx) = oneshot::channel();

        self.braid_tx
            .send(BraidCommand::GetCohortCount { respond_to: tx })
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;

        let count = rx
            .await
            .map_err(|_| ErrorObjectOwned::owned(2, "Internal error", None::<()>))?;
        Ok(count.to_string())
    }
}
//server building
//running a server in seperate spawn event
pub async fn run_rpc_server(braid_tx: mpsc::Sender<BraidCommand>) -> Result<SocketAddr, ()> {
    //Initializing the middleware
    let rpc_middleware = jsonrpsee::server::middleware::rpc::RpcServiceBuilder::new();
    //building the context/server supporting the http transport and ws
    let server = jsonrpsee::server::Server::builder()
        .set_rpc_middleware(rpc_middleware)
        .build("127.0.0.1:0")
        .await
        .unwrap();
    //listening address for incoming requests/connection
    let addr = server.local_addr().unwrap();
    //context for the served server
    let rpc_impl = RpcServerImpl::new(braid_tx);
    let handle = server.start(rpc_impl.into_rpc());
    log::info!(
        "RPC Server is listening at socket address http://{:?}",
        addr
    );
    tokio::spawn(
        //handling the stopping of the server
        handle.stopped(),
    );
    Ok(addr)
}

// Helper function to create test beads
pub fn create_test_bead(nonce: u32, prev_hash: Option<BlockHash>) -> Bead {
    let public_key = "020202020202020202020202020202020202020202020202020202020202020202"
        .parse::<bitcoin::PublicKey>()
        .unwrap();
    let time_hash_set = TimeVec(Vec::new());
    let mut parent_hash_set: HashSet<BlockHash> = HashSet::new();
    if let Some(hash) = prev_hash {
        parent_hash_set.insert(hash);
    }
    let weak_target = CompactTarget::from_consensus(32);
    let min_target = CompactTarget::from_consensus(1);
    let time_val = Time::from_consensus(1653195600).unwrap();
    let test_committed_metadata: CommittedMetadata = CommittedMetadata {
        comm_pub_key: public_key,
        min_target: min_target,
        miner_ip: "".to_string(),
        transactions: vec![],
        parents: parent_hash_set,
        parent_bead_timestamps: time_hash_set,
        payout_address: String::from(""),
        start_timestamp: time_val,
        weak_target: weak_target,
    };
    let extra_nonce = 42;
    let hex = "3046022100839c1fbc5304de944f697c9f4b1d01d1faeba32d751c0f7acb21ac8a0f436a72022100e89bd46bb3a5a62adc679f659b7ce876d83ee297c7a5587b2011c4fcc72eab45";
    let sig = Signature {
        signature: secp256k1::ecdsa::Signature::from_str(hex).unwrap(),
        sighash_type: EcdsaSighashType::All,
    };
    let test_uncommitted_metadata = UnCommittedMetadata {
        broadcast_timestamp: time_val,
        extra_nonce: extra_nonce,
        signature: sig,
    };
    let test_bytes: [u8; 32] = [0u8; 32];
    let test_block_header = BlockHeader {
        version: BlockVersion::TWO,
        prev_blockhash: prev_hash.unwrap_or(BlockHash::from_byte_array(test_bytes)),
        bits: CompactTarget::from_consensus(32),
        nonce: nonce,
        time: BlockTime::from_u32(8328429),
        merkle_root: TxMerkleNode::from_byte_array(test_bytes),
    };
    Bead {
        block_header: test_block_header,
        committed_metadata: test_committed_metadata,
        uncommitted_metadata: test_uncommitted_metadata,
    }
}
