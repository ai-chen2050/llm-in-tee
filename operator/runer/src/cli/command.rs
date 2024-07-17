use tracing::*;
use db_sql::pg::pg_client::setup_db;
use alloy_wrapper::util::generate_eth_account;

pub async fn init_db(postgres_conn_str: String) -> bool {
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

pub fn eth_account() -> bool {
    let (pri_key, pub_key, addr) = generate_eth_account();
    let pri_hex = hex::encode(pri_key);
        println!(
            "\nNew eth account: \nprikey: {} \npubkey: {} \naddress: {}",
            pri_hex, pub_key, addr
        );
    true
}