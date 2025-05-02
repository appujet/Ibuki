use axum::{body::Body, extract::Request, http::Response, http::StatusCode, middleware::Next};

pub async fn authenticate(request: Request, next: Next) -> Response<Body> {
    let Some(authorization) = request
        .headers()
        .get("Authorization")
        .map(|data| data.to_str().unwrap())
    else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("Missing Authorization in headers"))
            .unwrap();
    };

    // todo: do auth checks here and change placeholder
    if authorization != "placeholder" {
        // todo: return status code unathorized
    }

    next.run(request).await
}
