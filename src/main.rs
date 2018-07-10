extern crate actix_web;
extern crate env_logger;
#[macro_use] extern crate tera;

use actix_web::{server, App, HttpRequest, HttpResponse};
use actix_web::middleware::Logger;
use tera::{Context, Tera};

struct AppState {
    template: Tera
}

fn index(req: HttpRequest<AppState>) -> HttpResponse {
    let context = Context::new();
    let rendered = req
        .state().template
        .render("index.html.tera", &context)
        .expect("wow template");

    HttpResponse::Ok().body(rendered)
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();


    let app = || {
        let tera: Tera = compile_templates!("templates/**/*");

        App::with_state(AppState { template: tera })
            .middleware(Logger::default())
            .resource("/", |r| r.f(index))
    };

    server::new(app)
        .bind("localhost:8088")
        .unwrap()
        .run()
}
