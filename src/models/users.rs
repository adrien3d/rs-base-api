use mongodb::{
    bson::{doc, oid::ObjectId},
    options::IndexOptions,
    Client, IndexModel,
};
use serde::{Deserialize, Serialize};

pub const REPOSITORY_NAME: &str = "users";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AuthReq {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct User {
    pub _id: ObjectId,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub password: String,
}

/// Creates an index on the "email" field to force the values to be unique.
pub async fn create_email_index(client: &Client, db_name: &str) {
    let options = IndexOptions::builder().unique(true).build();
    let model = IndexModel::builder()
        .keys(doc! { "email": 1 })
        .options(options)
        .build();
    client
        .database(db_name)
        .collection::<User>(REPOSITORY_NAME)
        .create_index(model, None)
        .await
        .expect("creating an index should succeed");
}
