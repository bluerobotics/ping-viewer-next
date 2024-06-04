use actix_web::{http::StatusCode, ResponseError};

use paperclip::actix::api_v2_errors;
use validator::ValidationErrors;

#[allow(dead_code)]
#[api_v2_errors(
    code = 400,
    description = "Bad Request: The client's request contains invalid or malformed data.",
    code = 500,
    description = "Internal Server Error: An unexpected server error has occurred."
)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Bad Request: {0}")]
    BadRequest(String),
    #[error("Internal Server Error: {0}")]
    Internal(String),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<ValidationErrors> for Error {
    fn from(error: ValidationErrors) -> Self {
        Self::BadRequest(error.to_string())
    }
}
