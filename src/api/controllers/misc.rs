use axum::{Extension, Json};
use metrics::{histogram, increment_counter};
use tokio::time::Instant;
use uuid::Uuid;

use crate::api::controllers::*;
use crate::api::requests::*;
use crate::api::responses::*;
use crate::api::*;
use crate::models::*;

pub async fn post_read_contract(
    Json(req): Json<ExecuteContractRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<ReadContractResponse>> {
    let start = Instant::now();

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

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "readContract");
    increment_counter!("requests_processed", "method" => "readContract");

    Ok(Json(tokens))
}

pub async fn post_encode_tvm_cell(
    Json(req): Json<EncodeParamRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<EncodedCellResponse>> {
    let start = Instant::now();

    let cell = ctx
        .ton_service
        .encode_tvm_cell(
            req.input_params
                .into_iter()
                .map(InputParam::from)
                .collect::<Vec<InputParam>>(),
        )
        .map(|cell| EncodedCellResponse { base64_cell: cell })?;

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "encodeTvmCell");
    increment_counter!("requests_processed", "method" => "encodeTvmCell");

    Ok(Json(cell))
}

pub async fn post_prepare_generic_message(
    Json(req): Json<PrepareMessageRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<UnsignedMessageHashResponse>> {
    let start = Instant::now();

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

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "prepareGenericMessage");
    increment_counter!("requests_processed", "method" => "prepareGenericMessage");

    Ok(Json(UnsignedMessageHashResponse {
        unsigned_message_hash: hex::encode(unsigned_message.hash()),
    }))
}

pub async fn post_send_signed_message(
    Json(req): Json<SignedMessageRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
) -> Result<Json<SignedMessageHashResponse>> {
    let start = Instant::now();

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

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "sendSignedMessage");
    increment_counter!("requests_processed", "method" => "sendSignedMessage");

    Ok(Json(res))
}

pub async fn post_send_generic_message(
    Json(req): Json<SendMessageRequest>,
    Extension(ctx): Extension<Arc<ApiContext>>,
    IdExtractor(service_id): IdExtractor,
) -> Result<Json<TransactionResponse>> {
    let start = Instant::now();

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

    let transaction = ctx
        .ton_service
        .prepare_and_send_signed_generic_message(
            &service_id,
            &req.sender_addr,
            &req.target_account_addr,
            req.execution_flag,
            req.value,
            req.bounce,
            &req.account_type,
            &req.custodians,
            function_details,
            req.id.unwrap_or_else(Uuid::new_v4),
        )
        .await
        .map(From::from);

    let elapsed = start.elapsed();
    histogram!("execution_time_seconds", elapsed, "method" => "sendGenericMessage");
    increment_counter!("requests_processed", "method" => "sendGenericMessage");

    Ok(Json(TransactionResponse::from(transaction)))
}
