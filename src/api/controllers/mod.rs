mod ton;
pub use self::ton::*;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::models::owners_cache::OwnersCache;
use crate::services::{AuthService, TonService};
use crate::settings::Config;
use crate::sqlx_client::SqlxClient;

#[derive(Clone)]
pub struct Context {
    pub tokens_service: Arc<dyn TonService>,
    pub auth_service: Arc<dyn AuthService>,
}
