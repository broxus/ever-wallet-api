use crate::{models::WhitelistedTokenFromDb, utils::token_wallets::models::TokenWalletVersion};

use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResubscribeResponse {}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReadContractResponse {
    pub object: serde_json::Value,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EncodedCellResponse {
    pub base64_cell: String,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnsignedMessageHashResponse {
    pub unsigned_message_hash: String,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignedMessageHashResponse {
    pub signed_message_hash: String,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetCallbackResponse {
    pub callback: String,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistedTokenResponse {
    pub name: String,
    pub address: String,
    pub version: String,
}

impl From<WhitelistedTokenFromDb> for WhitelistedTokenResponse {
    fn from(t: WhitelistedTokenFromDb) -> Self {
        Self {
            name: t.name,
            address: t.address,
            version: TokenWalletVersion::from(t.version).to_string(),
        }
    }
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenWhitelistResponse {
    pub count: i32,
    pub items: Vec<WhitelistedTokenResponse>,
}
