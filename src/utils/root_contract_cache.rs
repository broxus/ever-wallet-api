use std::collections::HashMap;
use std::sync::Arc;

use nekoton::transport::models::ExistingContract;
use parking_lot::RwLock;
use ton_block::MsgAddressInt;

pub type RootContractCache = Arc<RwLock<HashMap<MsgAddressInt, ExistingContract>>>;
