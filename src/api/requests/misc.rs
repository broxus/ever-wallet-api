use bigdecimal::BigDecimal;
use opg::OpgModel;
use serde::Deserialize;
use ton_abi::Param;
use uuid::Uuid;

use crate::models::*;

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteContractRequest {
    pub target_account_addr: String,
    pub function_details: FunctionDetailsDTO,
    pub responsible: Option<bool>,
}

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDetailsDTO {
    pub function_name: String,
    pub input_params: Vec<InputParamDTO>,
    #[opg(string, format = "any[]")]
    pub output_params: Vec<Param>,
    #[opg(string, format = "any[]")]
    pub headers: Vec<Param>,
}

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct InputParamDTO {
    #[opg(string, format = "any")]
    pub param: Param,
    #[opg(string, format = "any")]
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

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct EncodeParamRequest {
    pub input_params: Vec<InputParamDTO>,
}

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct PrepareMessageRequest {
    pub sender_addr: String,
    pub public_key: String,
    pub target_account_addr: String,
    pub execution_flag: u8,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub bounce: bool,
    pub account_type: AccountType,
    pub custodians: Option<i32>,
    pub function_details: Option<FunctionDetailsDTO>,
}

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct SignedMessageRequest {
    pub sender_addr: String,
    pub hash: String,
    pub signature: String,
}

#[derive(Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub id: Option<Uuid>,
    pub sender_addr: String,
    pub target_account_addr: String,
    pub execution_flag: u8,
    #[opg("value", string)]
    pub value: BigDecimal,
    pub bounce: bool,
    pub account_type: AccountType,
    pub custodians: Option<i32>,
    pub function_details: Option<FunctionDetailsDTO>,
}
