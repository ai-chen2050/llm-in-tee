use crate::api::read::{index, status};
use crate::api::write::question;
use actix_web::web;

// static MESSAGE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn router(cfg: &mut web::ServiceConfig) {
    cfg.service(index);
    cfg.service(status);
    cfg.service(question);
}
