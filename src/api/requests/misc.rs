use bigdecimal::BigDecimal;

use schemars::JsonSchema;
use serde::Deserialize;
use ton_abi::Param;
use uuid::Uuid;

use crate::api::any_schema;
use crate::models::*;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteContractRequest {
    pub target_account_addr: String,
    pub function_details: FunctionDetailsDTO,
    pub responsible: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDetailsDTO {
    pub function_name: String,
    pub input_params: Vec<InputParamDTO>,
    #[schemars(schema_with = "any_schema")]
    pub output_params: Vec<Param>,
    #[schemars(schema_with = "any_schema")]
    pub headers: Vec<Param>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputParamDTO {
    #[schemars(schema_with = "any_schema")]
    pub param: Param,
    pub value: serde_json::Value,
}

impl From<InputParamDTO> for InputParam {
    fn from(i: InputParamDTO) -> Self {
        Self {
            param: i.param,
            value: i.value,
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EncodeParamRequest {
    pub input_params: Vec<InputParamDTO>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrepareMessageRequest {
    pub sender_addr: String,
    pub public_key: String,
    pub target_account_addr: String,
    pub execution_flag: u8,

    pub value: BigDecimal,
    pub bounce: bool,
    pub account_type: AccountType,
    pub custodians: Option<i32>,
    pub function_details: Option<FunctionDetailsDTO>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SignedMessageRequest {
    pub sender_addr: String,
    pub hash: String,
    pub signature: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub id: Option<Uuid>,
    pub sender_addr: String,
    pub target_account_addr: String,
    pub execution_flag: u8,

    pub value: BigDecimal,
    pub bounce: bool,
    pub account_type: AccountType,
    pub custodians: Option<i32>,
    pub function_details: Option<FunctionDetailsDTO>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetCallbackRequest {
    pub callback: String,
}
