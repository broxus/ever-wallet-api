use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tokio::sync::oneshot;
use tracing::log;

use crate::api::controllers::ControllersError;
use crate::client::TonClientError;
use crate::services::TonServiceError;

/// A common error type that can be used throughout the API.
///
/// Can be returned in a `Result` from an API handler function.
///
/// For convenience, this represents both API errors as well as internal recoverable errors,
/// and maps them to appropriate status codes along with at least a minimally useful error
/// message in a plain text body, or a JSON body in the case of `UnprocessableEntity`.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Automatically return `500 Internal Server Error` on a `sqlx::Error`.
    ///
    /// Via the generated `From<sqlx::Error> for Error` impl,
    /// this allows using `?` on database calls in handler functions without a manual mapping step.
    ///
    /// I highly recommend creating an error type like this if only to make handler function code
    /// nicer; code in Actix-web projects that we started before I settled on this pattern is
    /// filled with `.map_err(ErrInternalServerError)?` which is a *ton* of unnecessary noise.
    ///
    /// The actual error message isn't returned to the client for security reasons.
    /// It should be logged instead.
    ///
    /// Note that this could also contain database constraint errors, which should usually
    /// be transformed into client errors (e.g. `422 Unprocessable Entity` or `409 Conflict`).
    /// See `ResultExt` below for a convenient way to do this.
    #[error("an error occurred with the database")]
    Sqlx(#[from] sqlx::Error),

    #[error("an error occurred with serde")]
    Serde(#[from] serde_json::Error),

    #[error("an error occurred with ed25519")]
    Ed25519(#[from] ed25519_dalek::ed25519::Error),

    #[error("an error occurred with oneshot channel")]
    RecvError(#[from] oneshot::error::RecvError),

    #[error("an error occurred with tokens")]
    TokensJson(#[from] nekoton_abi::TokensJsonError),

    #[error("an error occurred with hex conversion")]
    FromHexError(#[from] hex::FromHexError),

    #[error("an error occurred with array conversion")]
    TryFromSliceError(#[from] std::array::TryFromSliceError),

    /// Return `500 Internal Server Error` on a `anyhow::Error`.
    ///
    /// `anyhow::Error` is used in a few places to capture context and backtraces
    /// on unrecoverable (but technically non-fatal) errors which could be highly useful for
    /// debugging. We use it a lot in our code for background tasks or making API calls
    /// to external services so we can use `.context()` to refine the logged error.
    ///
    /// Via the generated `From<anyhow::Error> for Error` impl, this allows the
    /// use of `?` in handler functions to automatically convert `anyhow::Error` into a response.
    ///
    /// Like with `Error::Sqlx`, the actual error message is not returned to the client
    /// for security reasons.
    #[error("an internal server error occurred: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    TonService(#[from] TonServiceError),

    #[error(transparent)]
    TonClient(#[from] TonClientError),

    #[error(transparent)]
    Controllers(#[from] ControllersError),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Sqlx(_)
            | Self::Serde(_)
            | Self::Anyhow(_)
            | Self::Ed25519(_)
            | Self::RecvError(_)
            | Self::TokensJson(_)
            | Self::FromHexError(_)
            | Self::TryFromSliceError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::TonService(e) => e.status_code(),
            Error::TonClient(e) => e.status_code(),
            Error::Controllers(e) => e.status_code(),
        }
    }
}
/// Axum allows you to return `Result` from handler functions, but the error type
/// also must be some sort of response type.
///
/// By default, the generated `Display` impl is used to return a plaintext error message
/// to the client.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::Sqlx(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("SQLx error: {:?}", e);
            }

            Self::Serde(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("Serde error: {:?}", e);
            }

            Self::Ed25519(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("Ed25519 error: {:?}", e);
            }

            Self::RecvError(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("RecvError error: {:?}", e);
            }

            Self::TokensJson(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("TokensJson error: {:?}", e);
            }

            Self::FromHexError(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("FromHexError error: {:?}", e);
            }

            Self::TryFromSliceError(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("TryFromSliceError error: {:?}", e);
            }

            Self::Anyhow(ref e) => {
                // TODO: we probably want to use `tracing` instead
                // so that this gets linked to the HTTP request by `TraceLayer`.
                log::error!("Generic error: {:?}", e);
            }

            // Other errors get mapped normally.
            Error::TonService(ref e) => {
                log::error!("Ton service error: {:?}", e);
            }

            // Other errors get mapped normally.
            Error::TonClient(ref e) => {
                log::error!("Ton client error: {:?}", e);
            }

            Error::Controllers(ref e) => {
                log::error!("Controllers error: {:?}", e);
            }
        }

        (
            self.status_code(),
            Json(serde_json::json!({"reason":self.to_string()})),
        )
            .into_response()
    }
}
