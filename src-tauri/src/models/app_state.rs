use crate::services::network_service::NetworkService;
use std::sync::{Arc, RwLock};

pub type SafeLock<T> = Arc<RwLock<T>>;

#[derive(Clone)]
pub struct AppState {
    pub network: SafeLock<NetworkService>, // I need a network services, but the engine can start delay?
}
