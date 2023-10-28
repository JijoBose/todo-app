//! Actix Web Diesel integration
//!
//! Diesel v2 is not an async library, so we have to execute queries in `web::block` closures which
//! offload blocking code (like Diesel's) to a thread-pool in order to not block the server.

#[macro_use]
extern crate diesel;

use actix_web::{error, get, middleware, post, web, App, HttpResponse, HttpServer, Responder};
use diesel::{prelude::*, r2d2};
use uuid::Uuid;

use crate::initdb::initialize_db_pool;

mod actions;
mod models;
mod schema;
mod initdb;

/// Short-hand for the database pool type to use throughout the app.
type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;

/// Get all tasks
#[get("/tasks")]
async fn get_all_tasks(pool: web::Data<DbPool>) -> actix_web::Result<impl Responder> {
    let tasks = web::block(move || {
        let mut conn = pool.get()?;
        actions::find_all_tasks(&mut conn)
    })
    .await?
    .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(tasks))
}

/// Finds task by UID.
#[get("/task/{task_id}")]
async fn get_task(
    pool: web::Data<DbPool>,
    task_uid: web::Path<Uuid>,
) -> actix_web::Result<impl Responder> {
    let task_uid = task_uid.into_inner();
    let task = web::block(move || {
        // note that obtaining a connection from the pool is also potentially blocking
        let mut conn = pool.get()?;
        actions::find_task_by_uid(&mut conn, task_uid)
    })
    .await?
    // map diesel query errors to a 500 error response
    .map_err(error::ErrorInternalServerError)?;

    Ok(match task {
        Some(task) => HttpResponse::Ok().json(task),
        None => HttpResponse::NotFound().body(format!("No task found with UID: {task_uid}")),
    })
}

/// Creates new task.
#[post("/task")]
async fn add_task(
    pool: web::Data<DbPool>,
    form: web::Json<models::NewTask>,
) -> actix_web::Result<impl Responder> {
    let task = web::block(move || {
        let mut conn = pool.get()?;

        actions::insert_new_task(&mut conn, &form.name, &form.done)
    })
    .await?
    .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Created().json(task))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // initialize DB pool outside of `HttpServer::new` so that it is shared across all workers
    let pool = initialize_db_pool();

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            // add DB pool handle to app data; enables use of `web::Data<DbPool>` extractor
            .app_data(web::Data::new(pool.clone()))
            // add request logger middleware
            .wrap(middleware::Logger::default())
            // add route handlers
            .service(get_task)
            .service(add_task)
            .service(get_all_tasks)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests;
