use crate::api::read::not_found;
use crate::handler::router;
use crate::operator::{ServerState, Operator, OperatorArc};
use crate::storage;
use node_api::config::OperatorConfig;
use node_api::error::OperatorResult;
use std::sync::Arc;
use actix_web::{middleware, web, App, HttpServer};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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

    pub async fn create_operator(config: OperatorConfig) -> OperatorArc {
        let cfg = Arc::new(config.clone());
        let node_id = config.node.node_id.clone().unwrap_or_default();
        let state = RwLock::new(ServerState::new(node_id, cfg.node.cache_msg_maximum));
        let storage = storage::Storage::new(cfg.clone()).await;
        let operator = Operator {
            config: cfg,
            storage,
            state,
        };

        Arc::new(operator)
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

    pub async fn initialize_node(self) -> OperatorResult<OperatorArc> {
        let arc_operator = OperatorFactory::create_operator(self.config.clone()).await;

        OperatorFactory::create_actix_node(arc_operator.clone()).await;

        Ok(arc_operator)
    }
}