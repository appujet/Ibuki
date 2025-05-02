use axum::{
    body::Body,
    extract::{Path, Request},
    http::Response,
    middleware::Next,
};

use crate::{constants::VERSION, util::errors::EndpointError};

pub async fn check(
    Path(version): Path<u8>,
    request: Request,
    next: Next,
) -> Result<Response<Body>, EndpointError> {
    if version != VERSION {
        return Err(EndpointError::UnprocessableEntity("Unsupported version"));
    }

    Ok(next.run(request).await)
}
