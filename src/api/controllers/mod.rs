use axum::response::IntoResponse;
use http::StatusCode;

pub use self::blockchain::*;
pub use self::address::*;
pub use self::authorization::*;
pub use self::docs::*;
pub use self::events::*;
pub use self::misc::*;
pub use self::ton_metrics::*;
pub use self::transactions::*;

mod blockchain;
mod address;
mod authorization;
mod docs;
mod events;
mod misc;
mod ton_metrics;
mod transactions;

pub async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND).into_response()
}

#[derive(thiserror::Error, Debug)]
pub enum ControllersError {
    #[error("Invalid request: `{0}`")]
    WrongInput(String),
}

impl ControllersError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ControllersError::WrongInput(_) => StatusCode::BAD_REQUEST,
        }
    }
}
