extern crate actix_web;
extern crate env_logger;
#[macro_use] extern crate log;
#[macro_use] extern crate tera;

use actix_web::{fs, server, App, HttpRequest, HttpResponse};
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
    std::env::set_var("RUST_LOG", "actix_todo=debug,actix_web=info");
    env_logger::init();

    let app = || {
        debug!("Compiling templates");
        let tera: Tera = compile_templates!("templates/**/*");

        debug!("Constructing the App");
        App::with_state(AppState { template: tera })
            .middleware(Logger::default())
            .resource("/", |r| r.f(index))
            .handler("/static", fs::StaticFiles::new("static/"))
    };

    debug!("Starting server");

    server::new(app)
        .bind("localhost:8088")
        .unwrap()
        .run()
}
