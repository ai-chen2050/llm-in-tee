use crate::{node_factory::OperatorFactory, storage::Storage};
use node_api::config::OperatorConfig;
use std::collections::{BTreeMap, VecDeque};
use std::{cmp, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::*;

pub struct Operator {
    pub config: Arc<OperatorConfig>,
    pub socket: UdpSocket,
    pub storage: Storage,
    pub state: RwLock<ServerState>,
}

pub type OperatorArc = Arc<Operator>;

impl Operator {
    pub fn operator_factory() -> OperatorFactory {
        OperatorFactory::init()
    }
}

/// A cache state of a server node.
#[derive(Debug, Clone)]
pub struct ServerState {
    // pub clock_info: ClockInfo,
    pub message_ids: VecDeque<String>,
    // pub cache_items: BTreeMap<String, ZMessage>,
    pub cache_maximum: u64,
}

impl ServerState {
    /// Create a new server state.
    pub fn new(node_id: String, cache_maximum: u64) -> Self {
        Self {
            message_ids: VecDeque::new(),
            cache_maximum,
        }
    }
}