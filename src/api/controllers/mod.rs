mod ton;
pub use self::ton::*;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::models::*;
use crate::services::*;
use crate::settings::*;
use crate::sqlx_client::*;

#[derive(Clone)]
pub struct Context {
    pub ton_service: Arc<dyn TonService>,
    pub auth_service: Arc<dyn AuthService>,
}
