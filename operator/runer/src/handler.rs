use crate::{
    operator::OperatorArc,
};
use crate::api::read::index;
use std::{
    net::SocketAddr, sync::Arc,
    // sync::atomic::{AtomicUsize, Ordering}, 
};
use actix_web::web;
use tracing::*;

// static MESSAGE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn router(cfg: &mut web::ServiceConfig) {
    cfg.service(index);
}