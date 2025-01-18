use crate::services::emails;
use anyhow::bail;
use async_trait::async_trait;
use lazy_static::lazy_static;
use mongodb::{error::ErrorKind, Client as mgoClient};
// use sqlx::postgres::PgPool;

use crate::models::users::{self, User};

pub mod mongo;
pub mod postgre;

lazy_static! {
    static ref DATABASE_NAME: String =
        std::env::var("DATABASE_NAME").unwrap_or_else(|_| "base-api".into());
}

#[derive(Debug)]
pub struct GenericDatabaseStatus {
    pub kind: String,
    pub is_connected: bool,
    pub migrations_performed: bool,
}

#[derive(Debug)]
pub struct MongoDatabase {
    pub status: GenericDatabaseStatus,
    pub client: Option<mgoClient>,
}

// pub struct PostgreDatabase {
//     status: GenericDatabaseStatus,
//     client: PgPool,
// }

#[async_trait]
pub trait GenericDatabase {
    fn new() -> Self;
    async fn connect(&mut self, uri: &str) -> anyhow::Result<&Self>;
    fn migrate(&self, reference: &str) -> Self;
}

impl MongoDatabase {
    pub async fn seed_user(&self, user: User) -> anyhow::Result<&Self> {
        match &self.client {
            Some(client) => {
                let collection = client
                    .database(&DATABASE_NAME)
                    .collection(users::REPOSITORY_NAME);
                match collection.insert_one(user.clone()).await {
                    Ok(_) => {
                        let _ = emails::send_email_with_aws_ses(&user.email, "Welcome", "Message")
                            .await;
                        Ok(self)
                    }
                    Err(error) => {
                        match *error.kind {
                            ErrorKind::Write(write_error) => {
                                match write_error {
                                mongodb::error::WriteFailure::WriteError(e) => {
                                    if e.code != 11000 {
                                        log::warn!("seed_user WriteFailure::WriteError: {e:?}");
                                    }
                                },
                                _ => bail!("Unknown writeConcernError while seed_user: {write_error:?}"),
                            }
                                Ok(self)
                            }
                            _ => bail!("Unknown errorKind while seed_user: {error}"),
                        }
                    }
                }
            }
            None => bail!("seed_user unable to get client"),
        }
    }

    // pub fn aggregate(&self) {
    //     log::info!("MongoDatabase aggregate");
    // }
}

#[async_trait]
impl GenericDatabase for MongoDatabase {
    fn new() -> Self {
        MongoDatabase {
            status: GenericDatabaseStatus {
                kind: "mongo".to_string(),
                is_connected: false,
                migrations_performed: false,
            },
            client: None,
        }
    }

    async fn connect(&mut self, uri: &str) -> anyhow::Result<&Self> {
        log::info!("Connecting to MongoDB with uri:{uri}");
        let mongo_db_client = mgoClient::with_uri_str(uri).await?;
        self.client = Some(mongo_db_client);
        self.status.is_connected = true;
        // MongoDatabase { status: (), client: mongo_db_client }
        Ok(self)
    }

    fn migrate(&self, _reference: &str) -> Self {
        todo!()
    }
}
