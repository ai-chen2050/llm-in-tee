use crate::api::read::not_found;
use crate::api::request::{answer_callback_task, periodic_heartbeat_task, register_worker};
use crate::handler::router;
use crate::operator::{Operator, OperatorArc, ServerState};
use crate::storage;
use actix_web::{middleware, web, App, HttpServer};
use alloy_primitives::hex::FromHex;
use alloy_primitives::B256;
use alloy_wrapper::contracts::vrf_range::new_vrf_range_backend;
use node_api::config::OperatorConfig;
use node_api::error::OperatorError;
use node_api::error::{
    OperatorError::{
        OPDecodeSignerKeyError, OPNewVrfRangeContractError
    }, 
    OperatorResult
};
use std::sync::Arc;
use tee_llm::nitro_llm::{tee_start_listening, try_connection, AnswerResp, PromptReq};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Default)]
pub struct OperatorFactory {
    pub config: OperatorConfig,
}

impl OperatorFactory {
    pub fn init() -> Self {
        Self::default()
    }

    pub fn set_config(mut self, config: OperatorConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn create_operator(
        config: OperatorConfig,
        tee_inference_sender: UnboundedSender<PromptReq>,
    ) -> OperatorResult<OperatorArc> {
        let cfg = Arc::new(config.clone());
        let node_id = config.node.node_id.clone();
        let signer_key = B256::from_hex(config.node.signer_key.clone()).map_err(OPDecodeSignerKeyError)?;
        let vrf_range_contract = 
                new_vrf_range_backend(&config.chain.chain_rpc_url, &config.chain.vrf_range_contract)
                .map_err(OPNewVrfRangeContractError)?;
        
        let server_state = ServerState::new(signer_key, node_id, cfg.node.cache_msg_maximum);
        let state = RwLock::new(server_state);
        let storage = storage::Storage::new(cfg.clone()).await;
        let operator = Operator {
            config: cfg,
            storage,
            state,
            tee_inference_sender,
            vrf_range_contract
        };

        Ok(Arc::new(operator))
    }

    async fn create_actix_node(arc_operator: OperatorArc) {
        let arc_operator_clone = Arc::clone(&arc_operator);

        let app = move || {
            App::new()
                .app_data(web::Data::new(arc_operator_clone.clone()))
                .wrap(middleware::Logger::default()) // enable logger
                .default_service(web::route().to(not_found))
                .configure(router)
        };

        HttpServer::new(app)
            .bind(arc_operator.config.net.rest_url.clone())
            .expect("Failed to bind address")
            .run()
            .await
            .expect("Failed to run server");
    }

    async fn prepare_setup(
        config: &OperatorConfig,
    ) -> OperatorResult<UnboundedSender<PromptReq>> {
        // detect and connect tee enclave service, if not, and exit
        let (prompt_sender, prompt_receiver) = unbounded_channel::<PromptReq>();
        let (answer_ok_sender, answer_ok_receiver) = unbounded_channel::<AnswerResp>();

        let (tee_cid, tee_port) = (config.net.tee_llm_cid, config.net.tee_llm_port);
        let result = try_connection(tee_cid, tee_port);
        if let Err(err) = result {
            return Err(OperatorError::OPConnectTEEError(err.to_string()));
        } else {
            info!("connect llm tee service successed!");
        }

        tokio::spawn(tee_start_listening(
            result.unwrap(),
            prompt_receiver,
            answer_ok_sender,
        ));

        // register status to dispatcher service
        let response = register_worker(config)
            .await
            .map_err(OperatorError::OPSetupRegister)?;

        if response.status().is_success() {
            info!(
                "register worker to dispatcher success! response_body: {:?}",
                response.text().await
            )
        } else {
            return Err(OperatorError::CustomError(format!(
                "Error: register to dispatcher failed, resp code {}",
                response.status()
            )));
        }

        // periodic heartbeat task
        let config_clone = config.clone();
        tokio::spawn(periodic_heartbeat_task(config_clone));
        
        // answer callback
        let config_clone = config.clone();
        tokio::spawn(answer_callback_task(config_clone, answer_ok_receiver));

        Ok(prompt_sender)
    }

    pub async fn initialize_node(self) -> OperatorResult<OperatorArc> {
        let prompt_sender =
            OperatorFactory::prepare_setup(&self.config).await?;

        let arc_operator = OperatorFactory::create_operator(
            self.config.clone(),
            prompt_sender,
        )
        .await?;

        OperatorFactory::create_actix_node(arc_operator.clone()).await;

        Ok(arc_operator)
    }
}
