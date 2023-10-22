//! Actix Web Diesel integration
//!
//! Diesel v2 is not an async library, so we have to execute queries in `web::block` closures which
//! offload blocking code (like Diesel's) to a thread-pool in order to not block the server.

#[macro_use]
extern crate diesel;

use actix_web::{error, get, middleware, post, web, App, HttpResponse, HttpServer, Responder};
use diesel::{prelude::*, r2d2};
use uuid::Uuid;

mod actions;
mod models;
mod schema;

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

/// Initialize database connection pool based on `DATABASE_URL` environment variable.
///
/// See more: <https://docs.rs/diesel/latest/diesel/r2d2/index.html>.
fn initialize_db_pool() -> DbPool {
    let conn_spec = std::env::var("DATABASE_URL").expect("DATABASE_URL should be set");
    let manager = r2d2::ConnectionManager::<SqliteConnection>::new(conn_spec);
    r2d2::Pool::builder()
        .build(manager)
        .expect("database URL should be valid path to SQLite DB file")
}

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test};

    use super::*;

    #[actix_web::test]
    async fn task_routes() {
        dotenv::dotenv().ok();
        env_logger::try_init_from_env(env_logger::Env::new().default_filter_or("info")).ok();

        let pool = initialize_db_pool();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool.clone()))
                .wrap(middleware::Logger::default())
                .service(get_task)
                .service(add_task),
        )
        .await;

        // send something that isn't a UUID to `get_task`
        let req = test::TestRequest::get().uri("/task/123").to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body = test::read_body(res).await;
        assert!(
            body.starts_with(b"UUID parsing failed"),
            "unexpected body: {body:?}",
        );

        // try to find a non-existent task
        let req = test::TestRequest::get()
            .uri(&format!("/task/{}", Uuid::nil()))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body = test::read_body(res).await;
        assert!(
            body.starts_with(b"No task found"),
            "unexpected body: {body:?}",
        );

        // create new task
        let req = test::TestRequest::post()
            .uri("/task")
            .set_json(models::NewTask::new("Test task", false))
            .to_request();
        let res: models::Task = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.name, "Test task");

        // get a task
        let req = test::TestRequest::get()
            .uri(&format!("/task/{}", res.id))
            .to_request();
        let res: models::Task = test::call_and_read_body_json(&app, req).await;
        assert_eq!(res.name, "Test task");

        // delete new task from table
        use crate::schema::tasks::dsl::*;
        diesel::delete(tasks.filter(id.eq(res.id)))
            .execute(&mut pool.get().expect("couldn't get db connection from pool"))
            .expect("couldn't delete test task from table");
    }
}
