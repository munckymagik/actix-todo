extern crate actix_web;
use actix_web::{server, App, HttpRequest, HttpResponse};

fn index(_req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body("Hello world!")
}

fn main() {
    server::new(|| App::new().resource("/", |r| r.f(index)))
        .bind("localhost:8088")
        .unwrap()
        .run()
}
