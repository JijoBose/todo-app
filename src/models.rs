use serde::{Deserialize, Serialize};
use crate::schema::tasks;

/// User details.
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = tasks)]
pub struct Task {
    pub id: String,
    pub name: String,
}

/// New user details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub name: String,
}

impl NewTask {
    /// Constructs new user details from name.
    #[cfg(test)] // only needed in tests
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
