use diesel::prelude::*;
use uuid::Uuid;

use crate::models::{self, Task};

type DbError = Box<dyn std::error::Error + Send + Sync>;

/// Query to get all tasks
pub fn find_all_tasks(conn: &mut SqliteConnection) -> Result<Vec<models::Task>, DbError> {
    use crate::schema::tasks::dsl::*;

    let get_tasks = tasks.load::<Task>(conn)?;
    Ok(get_tasks)
}

/// Run query using Diesel to find task by uid and return it.
pub fn find_task_by_uid(
    conn: &mut SqliteConnection,
    uid: Uuid,
) -> Result<Option<models::Task>, DbError> {
    use crate::schema::tasks::dsl::*;

    let task = tasks
        .filter(id.eq(uid.to_string()))
        .first::<models::Task>(conn)
        .optional()?;

    Ok(task)
}

/// Run query using Diesel to insert a new database row and return the result.
pub fn insert_new_task(
    conn: &mut SqliteConnection,
    nm: &str,
    dn: &bool,
) -> Result<models::Task, DbError> {
    // It is common when using Diesel with Actix Web to import schema-related
    // modules inside a function's scope (rather than the normal module's scope)
    // to prevent import collisions and namespace pollution.
    use crate::schema::tasks::dsl::*;

    let new_task = models::Task {
        id: Uuid::new_v4().to_string(),
        name: nm.to_owned(),
        done: *dn,
    };

    diesel::insert_into(tasks).values(&new_task).execute(conn)?;

    Ok(new_task)
}
