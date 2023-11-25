use actix_web::{middleware::Logger, web, App, HttpServer};
use api::tasks::{add_task, delete_task, get_all_tasks, get_task};

mod api;
mod actions;
mod model;
mod schema;
mod initdb;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    // initialize DB pool outside of `HttpServer::new` so that it is shared across all workers
    let pool = initdb::initialize_db_pool();

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        let logger = Logger::default();
        App::new()
            // add DB pool handle to app data; enables use of `web::Data<DbPool>` extractor
            .app_data(web::Data::new(pool.clone()))
            // add request logger middleware
            .wrap(logger)
            // add route handlers
            .service(get_task)
            .service(add_task)
            .service(get_all_tasks)
            .service(delete_task)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests;
