use serde::{Deserialize, Serialize};

pub const REPOSITORY_NAME: &str = "users";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct User {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}