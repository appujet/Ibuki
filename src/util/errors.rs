use axum::http;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Base64DecodeError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Unknown version detected. Got {0}")]
    UnknownVersion(u8)
}

#[derive(Error, Debug)]
pub enum EndpointError<'a> {
    #[error("Authentication Required")]
    AuthenticationRequired,
    #[error("Bad Request")]
    BadRequest,
    #[error("Unprocessable Entity")]
    UnprocessableEntity,
    #[error("Internal Server Error")]
    InternalServerError,
    #[error("Required option {0} missing in headers")]
    MissingOption(&'a str),
    #[error(transparent)]
    Base64Decode(#[from] Base64DecodeError),
    #[error(transparent)]
    ToStr(#[from] http::header::ToStrError),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
}

impl IntoResponse for EndpointError<'_> {
    fn into_response(self) -> Response {
        todo!()
    }
}