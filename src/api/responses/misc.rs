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
