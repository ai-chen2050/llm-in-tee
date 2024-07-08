use crate::api::read::{index, status};
use crate::operator::OperatorArc;
use actix_web::web;
use tracing::*;

// static MESSAGE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn router(cfg: &mut web::ServiceConfig) {
    cfg.service(index);
    cfg.service(status);
}
