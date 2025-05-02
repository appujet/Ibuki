use axum::{body::Body, extract::Request, http::Response, middleware::Next};

use crate::util::errors::EndpointError;

pub async fn authenticate(request: Request, next: Next) -> Result<Response<Body>, EndpointError> {
    let authorization = request
        .headers()
        .get("Authorization")
        .ok_or(EndpointError::MissingOption("Authorization"))?
        .to_str()?;

    if authorization != "placeholder" {
        // todo: return status code unathorized
    }

    Ok(next.run(request).await)
}
