use actix_web::web::Json;
use actix_web::{
    delete, error, get, post, web, HttpResponse, Responder, Result,
};
use diesel::{r2d2, SqliteConnection};
use serde::Serialize;
use uuid::Uuid;

use crate::actions;
use crate::model::task;

type DbPool = r2d2::Pool<r2d2::ConnectionManager<SqliteConnection>>;

#[derive(Serialize)]
struct Response {
    message: String,
}

/// Get all tasks
#[get("/tasks")]
async fn get_all_tasks(pool: web::Data<DbPool>) -> Result<impl Responder> {
    let tasks = web::block(move || {
        let mut conn = pool.get()?;
        actions::find_all_tasks(&mut conn)
    })
    .await?
    .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(tasks))
}

#[delete("/tasks/{task_id}")]
async fn delete_task(pool: web::Data<DbPool>, task_uid: web::Path<Uuid>,) -> Result<Json<Response>> {
  let uid: Uuid = task_uid.clone();
  let conn_result = pool.get();

  match conn_result {
    Ok(mut conn) => {
      match actions::destroy_task(&mut conn, uid) {
        Ok(_rows_deleted) => {
          Ok(web::Json(Response { message: "deleted".to_string()}))
        }
        Err(err) => {
          Ok(web::Json(Response { message: err.to_string() }))
        }
      }
    }
    Err(err) => {
      Ok(web::Json(Response { message: err.to_string() }))
    }
  }
}

/// Finds task by UID.
#[get("/task/{task_id}")]
async fn get_task(
    pool: web::Data<DbPool>,
    task_uid: web::Path<Uuid>,
) -> Result<impl Responder> {
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
    form: web::Json<task::NewTask>,
) -> Result<impl Responder> {
    let task = web::block(move || {
        let mut conn = pool.get()?;

        actions::insert_new_task(&mut conn, &form.name, &form.done)
    })
    .await?
    .map_err(error::ErrorInternalServerError)?;

    Ok(HttpResponse::Created().json(task))
}
