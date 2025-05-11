use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Path, Request},
    http::Response,
    middleware::Next,
};

use crate::{constants::VERSION, util::errors::EndpointError};

pub async fn check(
    Path(params): Path<HashMap<String, String>>,
    request: Request,
    next: Next,
) -> Result<Response<Body>, EndpointError> {
    if params
        .get("version")
        .ok_or(EndpointError::UnprocessableEntity("Unsupported version"))?
        .as_str()
        != VERSION.to_string().as_str()
    {
        return Err(EndpointError::UnprocessableEntity("Unsupported version"));
    }

    Ok(next.run(request).await)
}
