use actix_web::web;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct Response {
    pub request_id: String,
    pub success: bool,
    pub reason: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize)]
pub struct RespStatus {
    pub node_id: String,
    pub cpu_percent: String,
    pub mem_total: String,
    pub mem_used: String,
}

pub fn make_resp_json(
    request_id: String,
    success: bool,
    reason: String,
    data: Value,
) -> web::Json<Response> {
    
    web::Json(Response {
        request_id,
        success,
        reason,
        data,
    })
}
