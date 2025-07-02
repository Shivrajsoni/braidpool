use crate::bead::Bead;
use crate::braid::{AddBeadStatus, Braid};
use crate::utils::BeadHash;
use std::collections::HashSet;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum BraidCommand {
    GetBead {
        hash: BeadHash,
        respond_to: oneshot::Sender<Option<Bead>>,
    },
    AddBead {
        bead: Bead,
        respond_to: oneshot::Sender<AddBeadStatus>,
    },
    GetTips {
        respond_to: oneshot::Sender<Vec<BeadHash>>,
    },
    GetBeadCount {
        respond_to: oneshot::Sender<usize>,
    },
    GetCohortCount {
        respond_to: oneshot::Sender<usize>,
    },
}

pub struct BraidManager {
    braid: Braid,
    beads_storage: Vec<Bead>, // Store actual bead objects
    command_rx: mpsc::Receiver<BraidCommand>,
}

impl BraidManager {
    pub fn new(genesis_beads: HashSet<Bead>) -> (Self, mpsc::Sender<BraidCommand>) {
        let (command_tx, command_rx) = mpsc::channel(100);
        let manager = Self {
            braid: Braid::new(genesis_beads),
            beads_storage: Vec::new(),
            command_rx,
        };
        (manager, command_tx)
    }

    pub async fn run(mut self) {
        while let Some(command) = self.command_rx.recv().await {
            match command {
                BraidCommand::GetBead { hash, respond_to } => {
                    // Look up the bead by hash in storage
                    let bead = self
                        .beads_storage
                        .iter()
                        .find(|bead| bead.block_header.block_hash() == hash)
                        .cloned();
                    let _ = respond_to.send(bead);
                }
                BraidCommand::AddBead { bead, respond_to } => {
                    let success = self.braid.extend(&bead);
                    let status = if success {
                        AddBeadStatus::BeadAdded
                    } else {
                        AddBeadStatus::InvalidBead
                    };
                    // Store the bead if it was added successfully
                    if matches!(status, AddBeadStatus::BeadAdded) {
                        self.beads_storage.push(bead);
                    }
                    let _ = respond_to.send(status);
                }
                BraidCommand::GetTips { respond_to } => {
                    let tips: Vec<BeadHash> = self
                        .braid
                        .tips
                        .iter()
                        .map(|&index| self.braid.beads[index].block_header.block_hash())
                        .collect();
                    let _ = respond_to.send(tips);
                }
                BraidCommand::GetBeadCount { respond_to } => {
                    let count = self.braid.beads.len();
                    let _ = respond_to.send(count);
                }
                BraidCommand::GetCohortCount { respond_to } => {
                    log::info!("Cohorts: {:?}", self.braid.cohorts);
                    let count = self.braid.cohorts.len();
                    let _ = respond_to.send(count);
                }
            }
        }
    }
}
