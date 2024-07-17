use crate::api::request::QuestionReq;
use crate::api::response::{make_resp_json, Response, WorkerStatus};
use crate::operator::OperatorArc;
use actix_web::{body, get, post, web, Error, HttpRequest, HttpResponse, Result};
use alloy_wrapper::util::recover_signer_alloy;
use node_api::error::ErrorCodes;
use node_api::error::{OperatorAPIError::APIFailToJson, OperatorError::OPSendPromptError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tee_llm::nitro_llm::PromptReq;
use tracing::{debug, info};

/// WRITE API
// question input a prompt, and async return success, the answer callback later
#[post("/api/v1/question")]
async fn question(
    quest: web::Json<QuestionReq>,
    op: web::Data<OperatorArc>,
) -> web::Json<Response> {
    info!("Receive request, body = {:?}", quest);

    // todo: validate parameter and  signature
    if !quest.signature.is_empty() && !quest.prompt_hash.is_empty() {
        let addr = recover_signer_alloy(quest.signature.clone(), &quest.prompt_hash);
        if let Err(err) = addr {
            info!("Validate signature error, detail = {:?}", err);
        } else {
            debug!("recovered addr : {:?}", addr.unwrap());
        }
    }

    let req = PromptReq {
        request_id: quest.request_id.clone(),
        model_name: format!("./{}", quest.model),
        prompt: quest.prompt.clone(),
        n_ctx: quest.params.n_ctx,
        n_predict: quest.params.max_tokens as usize,
        n_threads: 4, // todo: use value in tee env
    };

    let result = op.tee_inference_sender.send(req);
    if let Err(err) = result {
        return make_resp_json(
            quest.request_id.clone(),
            ErrorCodes::OP_SEND_PROMPT_ERROR,
            OPSendPromptError(err.to_string()).to_string(),
            serde_json::Value::default(),
        );
    }
    let json_data = json!({});
    make_resp_json(quest.request_id.clone(), 0, String::new(), json_data)
}