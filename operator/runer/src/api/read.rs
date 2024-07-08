use std::str::FromStr;

use crate::api::response::{RespStatus, Response, make_resp_json};
use crate::operator::OperatorArc;
use actix_web::{body, get, post, web, Error, HttpRequest, HttpResponse, Result};
use node_api::error::OperatorAPIError::APIFailToJson;
use serde::{Deserialize, Serialize};
use sysinfo::System;

pub async fn not_found(_: web::Data<OperatorArc>, request: HttpRequest) -> String {
    format!("Not support api {}", request.uri())
}

/// GET API ZONE

#[get("/")]
async fn index() -> String {
    format!("Welcome to visit aos operator node's API!")
}

#[get("/v1/status")]
async fn status(_req: HttpRequest, op: web::Data<OperatorArc>) -> web::Json<Response> {
    let mut system: System = System::new_all();
    system.refresh_all();

    let cpu_percent = system.global_cpu_info().cpu_usage();
    let memory_total = system.total_memory();
    let memory_used = system.used_memory();

    let resp_data = RespStatus {
        node_id: op.config.node.node_id.clone().unwrap_or_default(),
        cpu_percent: format!("{:.2}%", cpu_percent),
        mem_total: format!("{} M", memory_total / 1024 / 1024),
        mem_used: format!("{} M", memory_used / 1024 / 1024),
    };
    
    let json_data = serde_json::to_value(&resp_data);

    match json_data {
        Err(_err) => {
            make_resp_json(String::new(), false, APIFailToJson.to_string(), serde_json::Value::default())
        }
        Ok(json_value) => {
            make_resp_json(String::new(), true, String::new(), json_value)
        }
    }
}

// POST API ZONE
// post json body, and deserialize `Info` from request's body
// #[post("/")]
// async fn index(info: web::Json<Status>) -> String {
//     format!("Welcome {} to aos operator's API!", info.username)
// }
