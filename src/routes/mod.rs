use actix_web::{HttpRequest, HttpResponse, Responder, get};

#[get("/")]
async fn landing(_: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello World ")
}

