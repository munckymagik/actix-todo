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
    dev::ResourceHandler, fs, http, server, App, AsyncResponder, Form, FutureResponse, HttpRequest,
    HttpResponse, Path, State,
};
use futures::{future, Future};
use tera::{Context, Tera};

mod db;
mod schema;
mod task;

struct AppState {
    template: Tera,
    db: Addr<Syn, db::Conn>,
}

#[derive(Deserialize)]
struct CreateForm {
    description: String,
}

#[derive(Deserialize)]
struct UpdateParams {
    id: i32,
}

#[derive(Deserialize)]
struct UpdateForm {
    _method: String,
}

macro_rules! send_and_then {
    ($db:expr, $message:expr, $block:expr) => {
        $db
            .send($message)
            .from_err()
            .and_then($block)
            .responder()
    };
}

macro_rules! send_then_redirect {
    ($db:expr, $message:expr) => {
        send_and_then!($db, $message, |res| match res {
            Ok(_) => Ok(HttpResponse::Found()
                .header(http::header::LOCATION, "/")
                .finish()),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
    };
}

fn index(state: State<AppState>) -> FutureResponse<HttpResponse> {
    send_and_then!(
        state.db,
        db::AllTasks,
        move |res| match res {
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
}

fn create((state, params): (State<AppState>, Form<CreateForm>)) -> FutureResponse<HttpResponse> {
    send_then_redirect!(
        state.db,
        db::CreateTask {
            description: params.description.clone()
        }
    )
}

fn update_or_delete(
    (state, params, form): (State<AppState>, Path<UpdateParams>, Form<UpdateForm>),
) -> FutureResponse<HttpResponse> {
    match form._method.as_ref() {
        "put" => send_then_redirect!(state.db, db::ToggleTask { id: params.id }),
        "delete" => send_then_redirect!(state.db, db::DeleteTask { id: params.id }),
        _ => future::ok(HttpResponse::BadRequest().into()).responder(),
    }
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
            .route("/", http::Method::GET, index)
            .resource("/todo/{id}", |r: &mut ResourceHandler<_>| {
                r.post().with(update_or_delete)
            })
            .route("/todo", http::Method::POST, create)
            .handler("/static", fs::StaticFiles::new("static/"))
            .default_resource(|r: &mut ResourceHandler<_>| r.f(not_found))
    };

    debug!("Starting server");

    server::new(app).bind("localhost:8088").unwrap().start();

    // Run actix system, this method actually starts all async processes
    let _ = system.run();
}
