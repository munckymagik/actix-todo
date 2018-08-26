use actix_web::{
    http, AsyncResponder, Form, FutureResponse, HttpRequest,
    HttpResponse, Path, State,
};
use actix_web::middleware::session::RequestSession;
use futures::{future, Future};
use tera::Context;

use AppState;
use handlers::{AllTasks, CreateTask, DeleteTask, ToggleTask};

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
pub struct CreateForm {
    description: String,
}

#[derive(Deserialize)]
pub struct UpdateParams {
    id: i32,
}

#[derive(Deserialize)]
pub struct UpdateForm {
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

fn redirect_to(location: &str) -> HttpResponse {
    HttpResponse::Found()
        .header(http::header::LOCATION, location)
        .finish()
}

fn bad_request() -> HttpResponse {
    HttpResponse::BadRequest().body("400 Bad Request")
}

pub fn not_found() -> HttpResponse {
    HttpResponse::NotFound().body("404 Not Found")
}

fn internal_server_error() -> HttpResponse {
    HttpResponse::InternalServerError().body("500 Internal Server Error")
}

pub fn handle_index(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    send_and_then!(req.state().db, AllTasks, move |res| match res {
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
                .expect("failed to render template");

            Ok(HttpResponse::Ok().body(rendered))
        }
        Err(_) => Ok(internal_server_error()),
    })
}

pub fn handle_create(
    (req, params): (HttpRequest<AppState>, Form<CreateForm>),
) -> FutureResponse<HttpResponse> {
    if params.description.is_empty() {
        flash!(req, Flash::error("Description cannot be empty"));
        future::ok(redirect_to("/")).responder()
    } else {
        send_and_then!(
            req.state().db,
            CreateTask {
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

pub fn handle_update_or_delete(
    (state, params, form): (State<AppState>, Path<UpdateParams>, Form<UpdateForm>),
) -> FutureResponse<HttpResponse> {
    match form._method.as_ref() {
        "put" => {
            send_and_then!(state.db, ToggleTask { id: params.id }, |res| match res {
                Ok(_) => Ok(redirect_to("/")),
                Err(_) => Ok(internal_server_error()),
            })
        },
        "delete" => {
            send_and_then!(state.db, DeleteTask { id: params.id }, |res| match res {
                Ok(_) => Ok(redirect_to("/")),
                Err(_) => Ok(internal_server_error()),
            })
        },
        _ => future::ok(bad_request()).responder(),
    }
}
