use std::collections::HashMap;
use std::sync::Arc;

use nekoton::transport::models::ExistingContract;
use ton_block::MsgAddressInt;

pub type RootStateCache = Arc<HashMap<MsgAddressInt, ExistingContract>>;
