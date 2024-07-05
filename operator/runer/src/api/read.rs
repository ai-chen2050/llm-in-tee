use actix_web::{post, body, web, Error, HttpRequest, HttpResponse};
use serde::{Serialize, Deserialize};
use crate::operator::OperatorArc;

pub async fn not_found(_: web::Data<OperatorArc>, request: HttpRequest) -> String {
    format!("Not support api {}!", request.uri())
}


#[derive(Serialize, Deserialize)]
struct Info {
    username: String,
}

/// deserialize `Info` from request's body
#[post("/")]
async fn index(info: web::Json<Info>) -> String {
    format!("Welcome {}!", info.username)
}

#[post("/{name}")]
async fn name(req: HttpRequest) -> web::Json<Info> {
    web::Json(Info {
        username: req.match_info().get("name").unwrap().to_owned(),
    })
}