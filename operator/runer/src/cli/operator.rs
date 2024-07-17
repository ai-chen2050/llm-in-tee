use crate::operator::Operator;
use crate::operator::OperatorArc;
use node_api::config;
use node_api::config::OperatorConfig;
use node_api::error::{ErrorCodes, OperatorConfigError};
use structopt::StructOpt;
use tracing::*;

use crate::cli::command::{eth_account, init_db};
use std::path::PathBuf;

#[derive(StructOpt)]
struct OperatorCli {
    #[structopt(
        short = "c",
        long = "config",
        parse(from_os_str),
        help = "Yaml file only"
    )]
    config_path: Option<std::path::PathBuf>,

    #[structopt(
        short = "i",
        long = "init_pg",
        help = "Init & refresh pg, caution: new db & new table"
    )]
    init_pg: Option<String>,

    #[structopt(
        short = "k",
        long = "eth_account",
        help = "Gen a eth account, and keypair"
    )]
    eth_account: bool,
}

pub async fn run_cli() {
    let mut help_info = true;
    let args = OperatorCli::from_args();

    // init pg db
    if let Some(pg_conn_str) = args.init_pg {
        help_info = false;
        info!("PostgreSQL connection addr: {}", pg_conn_str);
        // Use the PostgreSQL connection string here for initialization
        if !init_db(pg_conn_str).await {
            return;
        }
    }

    // gen eth account
    if args.eth_account {
        help_info = false;
        info!("Gen a eth account, and keypair");
        // Use the PostgreSQL connection string here for initialization
        if !eth_account() {
            return;
        }
    }

    // setup node
    if let Some(config_path) = args.config_path {
        help_info = false;
        let operator_config = construct_node_config(config_path.clone());

        let _operator = build_operator(operator_config.clone()).await;
    }

    if help_info {
        info!("\nPlease exec: operator -h for help info.\n")
    }
}

pub fn construct_node_config(config_path: PathBuf) -> config::OperatorConfig {
    match config::OperatorConfig::load_config(config_path) {
        Err(OperatorConfigError::ConfigMissing(_)) => {
            error!("config path can't found.");
            std::process::exit(ErrorCodes::PROCESS_EXIT);
        }
        Err(OperatorConfigError::SerializationError(_)) => {
            error!("config file can't be serialize, bad yaml format or incomplete field");
            std::process::exit(ErrorCodes::PROCESS_EXIT);
        }
        Err(OperatorConfigError::IllegalNodeId) => {
            error!("nodeid illegal, must be hex format, and 64 bits");
            std::process::exit(ErrorCodes::PROCESS_EXIT);
        }
        result => result.expect("failed to load zhronod config"),
    }
}

pub async fn build_operator(config: OperatorConfig) -> OperatorArc {
    Operator::operator_factory()
        .set_config(config)
        .initialize_node()
        .await
        .map_err(|e| {
            panic!("Failed to build operator due to error, detail {:?}", e);
        })
        .unwrap()
}
