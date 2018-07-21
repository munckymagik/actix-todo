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
use actix_web::middleware::session::{CookieSessionBackend, RequestSession, SessionStorage};
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

#[derive(Deserialize, Serialize)]
struct Flash {
    kind: String,
    message: String,
}

impl Flash {
    fn success(message: &str) -> Self {
        Self {
            kind: "success".to_owned(),
            message: message.to_owned(),
        }
    }

    fn error(message: &str) -> Self {
        Self {
            kind: "error".to_owned(),
            message: message.to_owned(),
        }
    }
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

macro_rules! flash {
    ($req:expr, $flash:expr) => {
        $req.session()
            .set("flash", $flash)
            .expect("failed to set cookie")
    };
}

macro_rules! send_and_then {
    ($db:expr, $message:expr, $block:expr) => {
        $db.send($message).from_err().and_then($block).responder()
    };
}

macro_rules! send_then_redirect {
    ($db:expr, $message:expr) => {
        send_and_then!($db, $message, |res| match res {
            Ok(_) => Ok(redirect_to("/")),
            Err(_) => Ok(internal_server_error()),
        })
    };
}

fn redirect_to(location: &str) -> HttpResponse {
    HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .finish()
}

fn bad_request() -> HttpResponse {
    HttpResponse::BadRequest().body("400 Bad Request")
}

fn not_found() -> HttpResponse {
    HttpResponse::NotFound().body("404 Not Found")
}

fn internal_server_error() -> HttpResponse {
    HttpResponse::InternalServerError().body("500 Internal Server Error")
}

fn index(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    send_and_then!(req.state().db, db::AllTasks, move |res| match res {
        Ok(tasks) => {
            let mut context = Context::new();
            context.add("tasks", &tasks);

            if let Some(flash) = req.session().get::<Flash>("flash")? {
                context.add("msg", &(flash.kind, flash.message));
                req.session().remove("flash");
            }

            let rendered = req.state()
                .template
                .render("index.html.tera", &context)
                .expect("wow template");

            Ok(HttpResponse::Ok().body(rendered))
        }
        Err(_) => Ok(internal_server_error()),
    })
}

fn create(
    (req, params): (HttpRequest<AppState>, Form<CreateForm>),
) -> FutureResponse<HttpResponse> {
    if params.description.is_empty() {
        flash!(req, Flash::error("Description cannot be empty"));
        future::ok(redirect_to("/")).responder()
    } else {
        send_and_then!(
            req.state().db,
            db::CreateTask {
                description: params.description.clone()
            },
            move |res| match res {
                Ok(_) => {
                    flash!(req, Flash::success("Task successfully added"));
                    Ok(redirect_to("/"))
                }
                Err(_) => Ok(internal_server_error()),
            }
        )
    }
}

fn update_or_delete(
    (state, params, form): (State<AppState>, Path<UpdateParams>, Form<UpdateForm>),
) -> FutureResponse<HttpResponse> {
    match form._method.as_ref() {
        "put" => send_then_redirect!(state.db, db::ToggleTask { id: params.id }),
        "delete" => send_then_redirect!(state.db, db::DeleteTask { id: params.id }),
        _ => future::ok(bad_request()).responder(),
    }
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
            .middleware(SessionStorage::new(
                CookieSessionBackend::signed(&[0; 32]).secure(false),
            ))
            .route("/", http::Method::GET, index)
            .resource("/todo/{id}", |r: &mut ResourceHandler<_>| {
                r.post().with(update_or_delete)
            })
            .route("/todo", http::Method::POST, create)
            .handler("/static", fs::StaticFiles::new("static/"))
            .default_resource(|r: &mut ResourceHandler<_>| r.f(|_| not_found()))
    };

    debug!("Starting server");

    server::new(app).bind("localhost:8088").unwrap().start();

    // Run actix system, this method actually starts all async processes
    let _ = system.run();
}
