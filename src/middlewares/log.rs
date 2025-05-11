use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Path, Request},
    http::Response,
    middleware::Next,
};

use crate::util::errors::EndpointError;

#[tracing::instrument]
pub async fn request(
    Path(params): Path<HashMap<String, String>>,
    request: Request,
    next: Next,
) -> Result<Response<Body>, EndpointError> {
    tracing::info!("Received a request! [Endpoint: {}]", request.uri());

    Ok(next.run(request).await)
}
