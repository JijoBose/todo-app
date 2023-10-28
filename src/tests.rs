use diesel::prelude::*;
use actix_web::http::StatusCode;
use actix_web::{middleware, web, App, test};
use uuid::Uuid;

// use crate::tests::initdb::initialize_db_pool;
use crate::{get_task, add_task, models};

mod initdb;

#[actix_web::test]
async fn task_routes() {
    dotenv::dotenv().ok();
    env_logger::try_init_from_env(env_logger::Env::new().default_filter_or("info")).ok();

    let pool = initdb::initialize_db_pool();

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
