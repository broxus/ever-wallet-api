use crate::models::WhitelistedTokenFromDb;
use nekoton_contracts::tip3_any::TokenWalletVersion;
use opg::OpgModel;
use serde::Serialize;

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct ReadContractResponse {
    #[opg(string, format = "any")]
    pub object: serde_json::Value,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct EncodedCellResponse {
    pub base64_cell: String,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct UnsignedMessageHashResponse {
    pub unsigned_message_hash: String,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct SignedMessageHashResponse {
    pub signed_message_hash: String,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct SetCallbackResponse {
    pub callback: String,
}

#[derive(Serialize, OpgModel)]
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

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("TokenWhitelistResponse")]
pub struct TokenWhitelistResponse {
    pub count: i32,
    pub items: Vec<WhitelistedTokenResponse>,
}
