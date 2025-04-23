use actix_web::{App, HttpServer};
use dotenv::dotenv;

mod routes;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    HttpServer::new(|| App::new().service(routes::landing))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
