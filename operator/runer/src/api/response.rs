use serde_json::Value;

pub struct Response {
    pub request_id: String,
    pub success: bool,
    pub reason: String,
    pub data: Value,
}