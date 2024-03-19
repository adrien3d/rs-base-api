use argon2::Config;
use json::JsonValue;
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

#[derive(Debug, Clone)]
pub struct RequestUser {
    pub user_id: ObjectId,
    pub org_id: ObjectId,
    pub name: String,
    pub email: String,
    //pub user_entity_ids: UserEntityList,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: String,
    pub admin: bool,
    pub exp: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct User {
    pub _id: ObjectId,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub org_id: Option<ObjectId>,
    pub email: String,
    pub password: String,
    //pub created: DateTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SanitizedUser {
    pub _id: ObjectId,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub org_id: Option<ObjectId>,
    pub email: String,
}

impl User {
    pub fn sanitize(&self) -> SanitizedUser {
        SanitizedUser {
            _id: self._id,
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            role: self.role.clone(),
            org_id: self.org_id,
            email: self.email.clone(),
        }
    }

    pub fn from_json_value(json: &JsonValue) -> Option<User> {
        // Extract the values from the JSON fields
        let first_name = json["first_name"].as_str()?.to_string();
        let last_name = json["last_name"].as_str()?.to_string();
        let role = json["role"].as_str()?.to_string();
        let email = json["email"].as_str()?.to_string();
        let password = json["password"].as_str()?;

        let mut org_object_id = ObjectId::new();
        match json["org_id"].as_str() {
            Some(org_id) => {
                if !org_id.is_empty() {
                    match mongodb::bson::oid::ObjectId::parse_str(org_id) {
                        Ok(oid) => org_object_id = oid,
                        Err(error) => log::error!("Org_id to Mongo Object ID fail: {error:?}"),
                    }
                }
            }
            None => {
                log::warn!("No organization id for: {email}");
                return None;
            }
        }

        let salt = &std::env::var("SECRET_KEY").unwrap_or_else(|_| "thisisasupersecretkey".into());
        let config = Config::default();
        let hashed_password =
            argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &config).unwrap();

        Some(User {
            _id: ObjectId::new(),
            first_name,
            last_name,
            role,
            org_id: Some(org_object_id),
            email,
            password: hashed_password,
        })
    }
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
