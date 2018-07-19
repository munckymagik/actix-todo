extern crate actix;
extern crate actix_web;
extern crate dotenv;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate tera;

use actix::prelude::{Addr, Syn, SyncArbiter};
use actix_web::middleware::Logger;
use actix_web::{
    dev::ResourceHandler, fs, Form, http, server, App, AsyncResponder, FutureResponse, HttpRequest,
    HttpResponse, State, Path
};
use futures::Future;
use tera::{Context, Tera};

mod db;
mod schema;
mod task;

struct AppState {
    template: Tera,
    db: Addr<Syn, db::Conn>,
}

#[derive(Deserialize)]
struct CreateParams {
    description: String,
}

#[derive(Deserialize)]
struct ToggleParams {
    id: i32,
}

fn index(state: State<AppState>) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(db::AllTasks)
        .from_err()
        .and_then(move |res| match res {
            Ok(tasks) => {
                let mut context = Context::new();
                context.add("tasks", &tasks);

                let rendered = state
                    .template
                    .render("index.html.tera", &context)
                    .expect("wow template");

                Ok(HttpResponse::Ok().body(rendered))
            }
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn create((state, params): (State<AppState>, Form<CreateParams>)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(db::CreateTask {
            description: params.description.clone()
        })
        .from_err()
        .and_then(|res| match res {
            Ok(_) => {
                Ok(HttpResponse::Found()
                    .header(http::header::LOCATION, "/")
                    .finish())
            },
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn toggle((state, params): (State<AppState>, Path<ToggleParams>)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(db::ToggleTask {
            id: params.id
        })
        .from_err()
        .and_then(|res| match res {
            Ok(_) => {
                Ok(HttpResponse::Found()
                    .header(http::header::LOCATION, "/")
                    .finish())
            },
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

fn not_found(_: HttpRequest<AppState>) -> HttpResponse {
    HttpResponse::NotFound().body("Not found")
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_todo=debug,actix_web=info");
    env_logger::init();

    // Start the Actix system
    let system = actix::System::new("todo-app");

    let pool = db::init_pool();
    let addr = SyncArbiter::start(3, move || db::Conn(pool.get().unwrap()));

    let app = move || {
        debug!("Compiling templates");
        let tera: Tera = compile_templates!("templates/**/*");

        debug!("Constructing the App");
        App::with_state(AppState {
            template: tera,
            db: addr.clone(),
        }).middleware(Logger::default())
            .resource("/", |r: &mut ResourceHandler<_>| {
                r.method(http::Method::GET).with(index)
            })
            .route("/todo", http::Method::POST, create)
            .resource("/todo/{id}", |r: &mut ResourceHandler<_>| {
                r.method(http::Method::POST).with(toggle)
            })
            .handler("/static", fs::StaticFiles::new("static/"))
            .default_resource(|r: &mut ResourceHandler<_>| r.f(not_found))
    };

    debug!("Starting server");

    server::new(app).bind("localhost:8088").unwrap().start();

    // Run actix system, this method actually starts all async processes
    let _ = system.run();
}
