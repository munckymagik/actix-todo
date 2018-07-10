extern crate actix_web;
extern crate env_logger;

use actix_web::{server, App, HttpRequest, HttpResponse};
use actix_web::middleware::Logger;

fn index(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body("Hello world!")
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let app = || {
        App::new()
            .middleware(Logger::default())
            .resource("/", |r| r.f(index))
    };

    server::new(app)
        .bind("localhost:8088")
        .unwrap()
        .run()
}
