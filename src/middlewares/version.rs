use axum::{
    body::Body,
    extract::{Path, Request},
    http::{Response, StatusCode},
    middleware::Next,
};

use crate::constants::VERSION;

pub async fn check(Path(version): Path<u8>, request: Request, next: Next) -> Response<Body> {
    if version != VERSION {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Unsupported Version"))
            .unwrap();
    }

    next.run(request).await
}
