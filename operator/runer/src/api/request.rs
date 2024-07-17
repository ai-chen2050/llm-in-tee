use crate::api::response::WorkerStatus;
use alloy_wrapper::util::sign_message;
use node_api::config::OperatorConfig;
use node_api::error::OperatorError;
use common::crypto::core::DigestHash;
use tee_llm::nitro_llm::AnswerResp;
use tools::helper::machine_used;
use alloy_primitives::hex::FromHex;
use alloy_primitives::B256;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};

#[derive(serde::Serialize)]
pub struct RegisterWorkerReq {
    pub worker_name: String,
    pub check_heart_beat: bool,
    pub worker_status: WorkerStatus,
    pub multimodal: bool,
}

#[derive(serde::Serialize)]
pub struct RegisterHeartbeatReq {
    pub worker_name: String,
    pub node_id: String,
    pub queue_length: u32,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct InferParams {
    pub n_ctx: u32,
    pub max_tokens: u32,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct QuestionReq {
    pub request_id: String,
    pub node_id: String,
    pub model: String,
    pub prompt: String,
    pub params: InferParams,
    pub prompt_hash: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize)]
pub struct AnswerCallbackReq {
    request_id: String,
    node_id: String,
    model: String,
    prompt: String,
    answer: String,
    elapsed: u64,
    attestation: String,
    attest_signature: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct HeartbeatResp {
    exist: bool,
}

pub async fn register_worker(config: &OperatorConfig) -> Result<reqwest::Response, reqwest::Error> {
    let (cpu_percent, memory_total, memory_used) = machine_used();
    let worker_status = WorkerStatus {
        node_id: config.node.node_id.clone(),
        model_names: config.node.ai_models.clone(),
        cpu_percent: format!("{:.2}%", cpu_percent),
        mem_total: format!("{} M", memory_total / 1024 / 1024),
        mem_used: format!("{} M", memory_used / 1024 / 1024),
        speed: 1,
        queue_length: 0,
    };

    let body = RegisterWorkerReq {
        worker_name: config.net.outer_url.clone(),
        check_heart_beat: true,
        worker_status,
        multimodal: false,
    };

    let client = ReqwestClient::new();
    client
        .post(format!(
            "{}{}",
            config.net.dispatcher_url.clone(),
            "/register_worker"
        ))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
}

async fn register_heartbeat(config: &OperatorConfig) -> Result<reqwest::Response, reqwest::Error> {
    debug!("Registering heartbeat to dispatcher...");

    let body = RegisterHeartbeatReq {
        worker_name: config.net.outer_url.clone(),
        node_id: config.node.node_id.clone(),
        queue_length: 0,
    };

    let client = ReqwestClient::new();
    client
        .post(format!(
            "{}{}",
            config.net.dispatcher_url.clone(),
            "/receive_heart_beat"
        ))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
}

pub async fn periodic_heartbeat_task(config: OperatorConfig) {
    let interval = Duration::from_secs(config.node.heartbeat_interval);
    loop {
        match register_heartbeat(&config).await {
            Ok(response) => {
                debug!("Response status: {}", response.status());
                match response.text().await {
                    Ok(body) => {
                        debug!("Response body: {}", body);
                        let json = serde_json::from_str(&body).unwrap_or_default();
                        let data: HeartbeatResp = serde_json::from_value(json).unwrap_or_default();
                        if !data.exist {
                            let response = register_worker(&config).await.map_err(OperatorError::OPSetupRegister).unwrap();
                            if response.status().is_success() {
                                info!(
                                    "register worker to dispatcher success! response_body: {:?}",
                                    response.text().await
                                )
                            }
                        }
                    },
                    Err(err) => error!("Failed to read response body, {}", err),
                }
            }
            Err(err) => error!("periodic heartbeat request error, {}", err),
        }
        sleep(interval).await;
    }
}

async fn answer_callback(
    config: &OperatorConfig,
    answer: &AnswerResp,
) -> Result<reqwest::Response, reqwest::Error> {
    debug!("answer callback to dispatcher. answer = {:?}", answer);
    use DigestHash as _;
    
    let mut sig_hex = String::new();
    let hex_attest = hex::encode(answer.document.0.clone());
    let signer_key = B256::from_hex(config.node.signer_key.clone());
    if let Ok(signer_key) = signer_key {
        let msg = hex_attest.sha256().to_fixed_bytes();
        let sig = sign_message(signer_key.0, msg).unwrap_or_default();
        sig_hex = sig.to_hex_bytes().to_string();
    }

    let body = AnswerCallbackReq {
        node_id: config.node.node_id.clone(),
        request_id: answer.request_id.clone(),
        model: answer.model_name.clone(),
        prompt: answer.prompt.clone(),
        answer: answer.answer.clone(),
        elapsed: answer.elapsed,
        attestation: hex_attest,
        attest_signature: sig_hex,
    };

    let client = ReqwestClient::new();
    client
        .post(format!(
            "{}{}",
            config.net.dispatcher_url.clone(),
            "/api/tee_callback"
        ))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
}

pub async fn answer_callback_task(
    config: OperatorConfig,
    mut receiver: UnboundedReceiver<AnswerResp>,
) {
    loop {
        if let Some(answer) = receiver.recv().await {
            match answer_callback(&config, &answer).await {
                Ok(response) => {
                    debug!("Response status: {}", response.status());
                    match response.text().await {
                        Ok(body) => debug!("Response body: {}", body),
                        Err(err) => error!("Failed to read response body, {}", err),
                    }
                }
                Err(err) => error!("answer callback request error, {}", err),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::{Client, Error};

    #[ignore = "local api"]
    #[tokio::test]
    async fn register() -> Result<(), Error> {
        let body = RegisterWorkerReq {
            worker_name: "http://localhost:21002".to_string(),
            check_heart_beat: true,
            worker_status: WorkerStatus {
                model_names: vec!["vicuna-7b-v1.5".to_string()],
                speed: 1,
                queue_length: 0,
                node_id: "todo!()".to_string(),
                cpu_percent: "todo!()".to_string(),
                mem_total: "todo!()".to_string(),
                mem_used: "todo!()".to_string(),
            },
            multimodal: false,
        };

        let client = Client::new();
        let response = client
            .post("http://127.0.0.1:21001/register_worker")
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await?;

        println!("Response status: {}", response.status());
        let response_body = response.text().await?;
        println!("Response body: {}", response_body);

        Ok(())
    }
}
