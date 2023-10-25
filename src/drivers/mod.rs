use anyhow::bail;
use async_trait::async_trait;
use mongodb::{
    error::{ErrorKind, WriteError},
    Client as mgoClient, Collection,
};
use sqlx::postgres::PgPool;

use crate::models::users::{self, User};

const DB_NAME: &str = "base-api";

pub mod mongo;
pub mod postgre;

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

pub struct PostgreDatabase {
    status: GenericDatabaseStatus,
    client: PgPool,
}

#[async_trait]
pub trait GenericDatabase {
    fn new() -> Self;
    async fn connect(&mut self, uri: &str) -> anyhow::Result<&Self>;
    fn migrate(&self, reference: &str) -> Self;
}

impl MongoDatabase {
    // pub fn new_with_client(&mut self, db_client: &mgoClient) {
    //     self.client = db_client.clone();
    // }
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
    // pub fn new_with_client(db_client: &mgoClient) -> Self {
    //     MongoDatabase {
    //         status: GenericDatabaseStatus {
    //             kind: "mongo".to_string(),
    //             is_connected: true,
    //             migrations_performed: false,
    //         },
    //         client: Some(db_client.clone()),
    //     }
    // }

    pub async fn seed_user(&self, user: User) -> anyhow::Result<&Self> {
        match &self.client {
            Some(client) => {
                let collection = client.database(DB_NAME).collection(users::REPOSITORY_NAME);
                match collection.insert_one(user, None).await {
                    Ok(_) => Ok(self),
                    Err(error) => {
                        let e = *error.kind;
                        bail!("seed_user insert error: {}", e);
                    }
                }
            }
            None => bail!("seed_user unable to get client"),
        }
    }

    pub fn aggregate(&self) {
        log::info!("MongoDatabase aggregate");
    }
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

    fn migrate(&self, reference: &str) -> Self {
        todo!()
    }
}
