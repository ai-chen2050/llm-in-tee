mod handler;
mod node_factory;
mod operator;
mod storage;
mod api;

use crate::operator::Operator;
use crate::operator::OperatorArc;
use db_sql::pg::pg_client::setup_db;
use node_api::config;
use node_api::config::OperatorConfig;
use node_api::error::{
    OperatorConfigError, ErrorCodes
};

use std::path::PathBuf;
use structopt::StructOpt;
use tools::tokio_static;
use tracing::*;
use tracing_subscriber::EnvFilter;

#[derive(StructOpt)]
struct OperatorCli {
    #[structopt(short = "c", long = "config", parse(from_os_str), help = "Yaml file only")]
    config_path: Option<std::path::PathBuf>,

    #[structopt(short = "i", long = "init_pg", help = "Init & refresh pg, caution: new db & new table")]
    init_pg: Option<String>,
}

fn main() {
    tokio_static::block_forever_on(async_main());
}

async fn async_main() {
    // set default log level: INFO
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(rust_log))
        .init();

    info!("start operator server");
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

async fn init_db(postgres_conn_str: String) -> bool {
    return if let Ok(url) = url::Url::parse(&postgres_conn_str) {
        let db_name = url.path().trim_start_matches('/');
        let base_url = url.as_str().trim_end_matches(db_name);
        let is_db_name_empty = db_name.is_empty();
        info!("Base URL: {}", base_url);
        info!("Database Name: {}", db_name);
        if is_db_name_empty {
            error!("Database name is empty, exiting");
            return false;
        }

        match setup_db(base_url, db_name).await {
            Err(err) => {
                error!("{}", err);
                false
            }
            Ok(conn) => {
                info!("Setup database success");
                let _ = conn.close().await;
                true
            }
        }
    } else {
        error!("Invalid PostgreSQL connection string");
        false
    };
}

fn construct_node_config(config_path: PathBuf) -> config::OperatorConfig {
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
        result => {
            result.expect("failed to load zhronod config")
        }
    }
}

async fn build_operator(config: OperatorConfig) -> OperatorArc {
    Operator::operator_factory().set_config(config).initialize_node().await.map_err(|e| {
        panic!("Failed to build operator due to error, detail {:?}", e);
    }).unwrap()
}
