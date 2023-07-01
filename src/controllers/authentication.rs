use crate::controllers::error::*;
use crate::models::users::{self};
use crate::DB_NAME;
use actix_web::{
    dev::ServiceRequest,
    http::StatusCode,
    post,
    web::{self, Json},
    HttpResponse, Responder,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mongodb::{
    bson::{doc, oid::ObjectId},
    Client, Collection,
};
use serde::{Deserialize, Serialize};

use std::{
    future::{ready, Ready},
    rc::Rc,
    str::FromStr,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub user_id: String,
    pub role: String,
    pub iat: usize,
    pub exp: usize,
}

#[post("/auth")]
pub(crate) async fn authentication(
    client: web::Data<Client>,
    req_body: web::Json<users::AuthReq>,
) -> impl Responder {
    //println!("{} {}", req_body.email, req_body.password);
    let secret_key = "supersecret"; //std::env::var("RSA_KEY");

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;

    let collection: Collection<users::User> =
        client.database(DB_NAME).collection(users::REPOSITORY_NAME);
    match collection
        .find_one(doc! { "email": &req_body.email.to_string() }, None)
        .await
    {
        Ok(Some(user)) => {
            let claims: TokenClaims = TokenClaims {
                user_id: user._id.to_string(),
                role: "admin".to_string(),
                exp,
                iat,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret_key.as_ref()),
            )
            .unwrap();

            return HttpResponse::Ok().json(token);
        }
        Ok(None) => {
            return HttpResponse::NotFound().body(format!(
                "No user found with email {}",
                &req_body.email.to_string()
            ));
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    };
    // let matching = verify(&user.hash, &auth_data.password);
    HttpResponse::Ok().json(req_body)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub status: String,
    pub message: String,
}

pub fn check_jwt(token: String) -> Result<TokenClaims, (StatusCode, Json<ErrorResponse>)> {
    let secret_key = "supersecret"; //std::env::var("RSA_KEY");

    let claims = decode::<TokenClaims>(
        &token,
        &DecodingKey::from_secret(secret_key.as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| {
        let json_error = ErrorResponse {
            status: "fail".to_string(),
            message: "Invalid token".to_string(),
        };
        (StatusCode::UNAUTHORIZED, Json(json_error))
    })?
    .claims;
    println!("claims: {:?}", claims);
    Ok(claims)
}

#[derive(Debug, Clone)]
pub enum AuthenticationInfo {
    /*ApiKey {
        key: api_key::ApiKeyAuth,
        user: RequestUser,
    },*/
    User(RequestUser),
}
#[derive(Clone, Debug)]
pub struct AppState {
    pub mongo_db: mongodb::Client,
    /// Temporary method of implementing admin user
    pub admin_user: Option<ObjectId>,
}

use actix_web::{FromRequest, HttpMessage, HttpRequest};
use chrono::{DateTime, Utc};
use tracing::{event, field, instrument, Level};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PermissionType {
    #[serde(rename = "trigger_event")]
    TriggerEvent,
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Permission {
    #[serde(rename = "permission_type")]
    pub perm: PermissionType,
    #[serde(rename = "permissioned_object")]
    pub object: Option<ObjectId>,
}

#[derive(Debug, Clone)]
pub struct User {
    pub user_id: ObjectId,
    pub external_user_id: String,
    pub active_org_id: ObjectId,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RequestUser {
    pub user_id: ObjectId,
    pub org_id: ObjectId,
    pub name: String,
    pub email: String,
    pub user_entity_ids: UserEntityList,
    pub is_admin: bool,
}

/// Extracts authentication information for routes that optionally require it.
pub struct MaybeAuthenticated(Option<Rc<AuthenticationInfo>>);

impl MaybeAuthenticated {
    pub fn into_inner(self) -> Option<Rc<AuthenticationInfo>> {
        self.0
    }

    pub fn expect_authed(self) -> Result<Rc<AuthenticationInfo>, Error> {
        self.0.ok_or(Error::AuthenticationError)
    }
}

impl FromRequest for MaybeAuthenticated {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let value = req.extensions().get::<Rc<AuthenticationInfo>>().cloned();
        ready(Ok(MaybeAuthenticated(value)))
    }
}

impl std::ops::Deref for MaybeAuthenticated {
    type Target = Option<Rc<AuthenticationInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extracts authentication information for routes that must be authenticated.
/// Returns an Error::AuthenticationError if the user is not authenticated.
#[derive(Debug)]
pub struct Authenticated(Rc<AuthenticationInfo>);

impl Authenticated {
    pub fn into_inner(self) -> Rc<AuthenticationInfo> {
        self.0
    }
}

impl FromRequest for Authenticated {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let value = req.extensions().get::<Rc<AuthenticationInfo>>().cloned();
        let result = match value {
            Some(v) => Ok(Authenticated(v)),
            None => Err(Error::AuthenticationError),
        };
        ready(result)
    }
}

impl std::ops::Deref for Authenticated {
    type Target = Rc<AuthenticationInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type UserEntityList = smallvec::SmallVec<[ObjectId; 4]>;

impl AuthenticationInfo {
    pub fn org_id(&self) -> &ObjectId {
        match self {
            Self::User(user) => &user.org_id,
            //Self::ApiKey { key, .. } => &key.org_id,
        }
    }

    pub fn user_id(&self) -> &ObjectId {
        match self {
            Self::User(user) => &user.user_id,
            //Self::ApiKey { user, .. } => &user.user_id,
        }
    }

    // pub fn user_entity_ids(&self) -> UserEntityList {
    //     match self {
    //         Self::User(user) => user.user_entity_ids.clone(),
    //         Self::ApiKey { key, user } => match (key.inherits_user_permissions, user) {
    //             (false, _) => {
    //                 let mut list = UserEntityList::new();
    //                 list.push(key.api_key_id);
    //                 list
    //             }
    //             (true, user) => {
    //                 let mut ids = user.user_entity_ids.clone();
    //                 ids.push(key.api_key_id);
    //                 ids
    //             }
    //         },
    //     }
    // }

    pub fn expect_admin(&self) -> Result<(), Error> {
        let is_admin = match self {
            Self::User(user) => user.is_admin,
            //Self::ApiKey { user, .. } => user.is_admin,
        };

        if is_admin {
            Ok(())
        } else {
            Err(Error::AuthorizationError)
        }
    }
}

impl AppState {
    pub fn new(mongo_db_client: mongodb::Client) -> Result<AppState, Error> {
        Ok(AppState {
            mongo_db: mongo_db_client,
            admin_user: envoption::optional("ADMIN_USER_ID")?,
        })
    }

    // Authenticate via cookie or API key, depending on what's provided.
    pub async fn authenticate(
        &self,
        identity: Option<actix_identity::Identity>,
        req: &ServiceRequest,
    ) -> Result<Option<AuthenticationInfo>, Error> {
        /*if let Some(auth) = api_key::get_api_key(self, req).await? {
            return Ok(Some(auth));
        }*/

        match identity {
            Some(identity) => {
                let user_id =
                    ObjectId::from_str(&identity.id().map_err(|_| Error::AuthenticationError)?)
                        .map_err(|_| Error::AuthenticationError)?;

                let req_user = self.get_user_info(&user_id).await?;
                Ok(Some(AuthenticationInfo::User(req_user)))
            }
            None => Ok(None),
        }
    }

    async fn get_user_info(&self, user_id: &ObjectId) -> Result<RequestUser, Error> {
        let collection: Collection<users::User> = self
            .mongo_db
            .database(DB_NAME)
            .collection(users::REPOSITORY_NAME);
        match collection.find_one(doc! { "_id": &user_id }, None).await {
            Ok(Some(user)) => Ok(RequestUser {
                user_id: user._id,
                name: user.last_name,
                email: user.email,
                is_admin: false,
                org_id: todo!(),
                user_entity_ids: todo!(),
            }),
            Ok(None) => todo!(),
            Err(err) => todo!(),
        }
        // let mut conn = self.pg.acquire().await?;
        // get_user_info(&mut conn, user_id, self.admin_user.as_ref()).await
    }
}

// pub async fn get_user_info(
//     tx: &mut PgConnection,
//     user_id: &UserId,
//     admin_user: Option<&UserId>,
// ) -> Result<RequestUser, Error> {
//     event!(Level::DEBUG, "Fetching user");
//     query!(
//         r##"SELECT user_id as "user_id: UserId",
//             active_org_id AS "org_id: OrgId", users.name, email,
//             array_agg(role_id) FILTER(WHERE role_id IS NOT NULL) AS "roles: Vec<RoleId>"
//         FROM users
//         JOIN orgs ON orgs.org_id = active_org_id
//         LEFT JOIN user_roles USING(user_id, org_id)
//         WHERE user_id = $1 AND NOT users.deleted AND NOT orgs.deleted
//         GROUP BY user_id"##,
//         &user_id.0
//     )
//     .fetch_optional(tx)
//     .await?
//     .map(|user| {
//         let user_entity_ids = match user.roles {
//             Some(roles) => {
//                 let mut ids = UserEntityList::with_capacity(roles.len() + 1);
//                 for role in roles {
//                     ids.push(role.into());
//                 }
//                 ids.push(user.user_id.clone().into());
//                 ids
//             }
//             None => UserEntityList::from_elem(user.user_id.clone().into(), 1),
//         };

//         let user = RequestUser {
//             user_id: user.user_id,
//             org_id: user.org_id,
//             name: user.name,
//             email: user.email,
//             //user_entity_ids,
//             is_admin: admin_user.map(|u| u == user_id).unwrap_or(false),
//         };

//         tracing::Span::current().record("user", &field::debug(&user));

//         user
//     })
//     .ok_or(Error::AuthenticationError)
// }
