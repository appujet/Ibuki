use axum::body::Body;
use axum::http::{self, StatusCode};
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlayerManagerError {
    #[error(transparent)]
    Connection(#[from] songbird::error::ConnectionError),
    #[error(transparent)]
    Control(#[from] songbird::error::ControlError),
    #[error("A connection is required to execute this action")]
    MissingConnection,
}

#[derive(Error, Debug)]
pub enum Base64DecodeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Unknown version detected. Got {0}")]
    UnknownVersion(u8),
}

#[derive(Error, Debug)]
pub enum Base64EncodeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Unknown version detected. Got {0}")]
    UnknownVersion(u32),
}

#[derive(Error, Debug)]
pub enum EndpointError {
    #[error("Required option {0} missing in headers")]
    MissingOption(&'static str),
    #[error("Unprocessable Entity due to: {0}")]
    UnprocessableEntity(&'static str),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    Base64Decode(#[from] Base64DecodeError),
    #[error(transparent)]
    Base64Encode(#[from] Base64EncodeError),
    #[error(transparent)]
    ToStr(#[from] http::header::ToStrError),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}

impl IntoResponse for EndpointError {
    #[tracing::instrument]
    fn into_response(self) -> Response<Body> {
        tracing::warn!(
            "Something Happened when processing this endpoint: {:?}",
            self
        );

        let tuple = match self {
            EndpointError::MissingOption(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            EndpointError::UnprocessableEntity(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string())
            }
            EndpointError::Base64Decode(base64_decode_error) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                base64_decode_error.to_string(),
            ),
            EndpointError::Base64Encode(base64_encode_error) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                base64_encode_error.to_string(),
            ),
            EndpointError::ToStr(to_str_error) => {
                (StatusCode::UNPROCESSABLE_ENTITY, to_str_error.to_string())
            }
            EndpointError::ParseInt(parse_int_error) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                parse_int_error.to_string(),
            ),
            EndpointError::JsonError(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
        };

        tuple.into_response()
    }
}
