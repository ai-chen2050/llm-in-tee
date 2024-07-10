use crate::api::response::WorkerStatus;
use node_api::config::OperatorConfig;
use reqwest::Client as ReqwestClient;
use tools::helper::machine_used;

#[derive(serde::Serialize)]
pub struct RegisterWorkerReq {
    pub worker_name: String,
    pub check_heart_beat: bool,
    pub worker_status: WorkerStatus,
    pub multimodal: bool,
}

pub async fn register_worker(config: &OperatorConfig) -> Result<reqwest::Response, reqwest::Error> {
    let (cpu_percent, memory_total, memory_used) = machine_used();

    let worker_status = WorkerStatus {
        node_id: config.node.node_id.clone().unwrap_or_default(),
        model_names: config.node.ai_models.clone(),
        cpu_percent: format!("{:.2}%", cpu_percent),
        mem_total: format!("{} M", memory_total / 1024 / 1024),
        mem_used: format!("{} M", memory_used / 1024 / 1024),
        speed: 1,
        queue_length: 0,
    };

    let body = RegisterWorkerReq {
        worker_name: config.net.dispatcher_url.clone(),
        check_heart_beat: true,
        worker_status,
        multimodal: false,
    };

    let client = ReqwestClient::new();
    client
        .post(format!("{}{}", config.net.dispatcher_url.clone(), "/register_worker"))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&body)
        .send()
        .await
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
