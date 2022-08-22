use axum::{Extension, Json};

use crate::axum_api::controllers::*;
use crate::axum_api::requests::*;
use crate::axum_api::responses::*;
use crate::axum_api::*;
use crate::models::*;

pub async fn post_read_contract(
    Json(req): Json<ExecuteContractRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<ReadContractResponse>> {
    let tokens = ctx
        .ton_service
        .execute_contract_function(
            &req.target_account_addr,
            &req.function_details.function_name,
            req.function_details
                .input_params
                .into_iter()
                .map(InputParam::from)
                .collect::<Vec<InputParam>>(),
            req.function_details.output_params,
            req.function_details.headers,
        )
        .await
        .map(|value| ReadContractResponse { object: value })?;

    Ok(Json(tokens))
}

pub async fn post_encode_tvm_cell(
    Json(req): Json<EncodeParamRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<EncodedCellResponse>> {
    let cell = ctx
        .ton_service
        .encode_tvm_cell(
            req.input_params
                .into_iter()
                .map(InputParam::from)
                .collect::<Vec<InputParam>>(),
        )
        .map(|cell| EncodedCellResponse { base64_cell: cell })?;

    Ok(Json(cell))
}

pub async fn post_prepare_generic_message(
    Json(req): Json<PrepareMessageRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<UnsignedMessageHashResponse>> {
    let function_details = req.function_details.map(|d| FunctionDetails {
        function_name: d.function_name,
        input_params: d
            .input_params
            .into_iter()
            .map(InputParam::from)
            .collect::<Vec<InputParam>>(),
        output_params: d.output_params,
        headers: d.headers,
    });

    let unsigned_message = ctx
        .ton_service
        .prepare_generic_message(
            &req.sender_addr,
            hex::decode(&req.public_key)?.as_slice(),
            &req.target_account_addr,
            req.execution_flag,
            req.value,
            req.bounce,
            &req.account_type,
            &req.custodians,
            function_details,
        )
        .await?;

    ctx.memory_storage.add_message(unsigned_message.clone());

    Ok(Json(UnsignedMessageHashResponse {
        unsigned_message_hash: hex::encode(unsigned_message.hash()),
    }))
}

pub async fn post_send_signed_message(
    Json(req): Json<SignedMessageRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<SignedMessageHashResponse>> {
    let res = match ctx.memory_storage.get_message(&req.hash) {
        Some(message) => {
            let signature: [u8; 64] = hex::decode(req.signature)
                .map_err(|_| ControllersError::WrongInput("Bad signature format".to_string()))?
                .try_into()
                .map_err(|_| ControllersError::WrongInput("Bad signature format".to_string()))?;

            let signed_message = message
                .sign(&signature)
                .map_err(|_| ControllersError::WrongInput("Bad signature format".to_string()))?;

            let hash = ctx
                .ton_service
                .send_signed_message(req.sender_addr, req.hash, signed_message)
                .await?;

            Ok(SignedMessageHashResponse {
                signed_message_hash: hash,
            })
        }
        None => Err(ControllersError::WrongInput(
            "Message unknown or expired".to_string(),
        )),
    }?;

    Ok(Json(res))
}
