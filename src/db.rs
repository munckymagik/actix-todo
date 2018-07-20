use std::env;
use std::ops::Deref;

use actix::prelude::{Actor, Handler, Message, SyncContext};
use actix_web::{error, Error};

use dotenv::dotenv;

use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

use task::{NewTask, Task};

type PgPool = Pool<ConnectionManager<PgConnection>>;

pub fn init_pool() -> PgPool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .expect("Failed to create pool")
}

pub struct Conn(pub PooledConnection<ConnectionManager<PgConnection>>);

impl Deref for Conn {
    type Target = PgConnection;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct CreateTask {
    pub description: String,
}

impl Message for CreateTask {
    type Result = Result<(), Error>;
}

pub struct AllTasks;

impl Message for AllTasks {
    type Result = Result<Vec<Task>, Error>;
}

pub struct ToggleTask {
    pub id: i32,
}

impl Message for ToggleTask {
    type Result = Result<(), Error>;
}

pub struct DeleteTask {
    pub id: i32,
}

impl Message for DeleteTask {
    type Result = Result<(), Error>;
}

impl Actor for Conn {
    type Context = SyncContext<Self>;
}

impl Handler<AllTasks> for Conn {
    type Result = Result<Vec<Task>, Error>;

    fn handle(&mut self, _: AllTasks, _: &mut Self::Context) -> Self::Result {
        Task::all(self).map_err(|_| error::ErrorInternalServerError("Error inserting task"))
    }
}

impl Handler<CreateTask> for Conn {
    type Result = Result<(), Error>;

    fn handle(&mut self, todo: CreateTask, _: &mut Self::Context) -> Self::Result {
        let new_task = NewTask {
            description: todo.description,
        };
        Task::insert(new_task, self)
            .map(|_| ())
            .map_err(|_| error::ErrorInternalServerError("Error inserting task"))
    }
}

impl Handler<ToggleTask> for Conn {
    type Result = Result<(), Error>;

    fn handle(&mut self, task: ToggleTask, _: &mut Self::Context) -> Self::Result {
        Task::toggle_with_id(task.id, self)
            .map(|_| ())
            .map_err(|_| error::ErrorInternalServerError("Error inserting task"))
    }
}

impl Handler<DeleteTask> for Conn {
    type Result = Result<(), Error>;

    fn handle(&mut self, task: DeleteTask, _: &mut Self::Context) -> Self::Result {
        Task::delete_with_id(task.id, self)
            .map(|_| ())
            .map_err(|_| error::ErrorInternalServerError("Error inserting task"))
    }
}
