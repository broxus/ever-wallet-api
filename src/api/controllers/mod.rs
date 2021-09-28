use std::sync::Arc;

use crate::services::*;

pub use self::ton::*;

mod ton;

#[derive(Clone)]
pub struct Context {
    pub ton_service: Arc<dyn TonService>,
    pub auth_service: Arc<dyn AuthService>,
}
