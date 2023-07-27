use crate::controllers::error::*;
use crate::models::users::{self, User};
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

use crate::controllers::error::Error::DatabaseError;
use std::{
    future::{ready, Ready},
    rc::Rc,
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
    let exp = (now + chrono::Duration::days(1)).timestamp() as usize;

    let collection: Collection<users::User> =
        client.database(DB_NAME).collection(users::REPOSITORY_NAME);
    match collection
        .find_one(doc! { "email": &req_body.email.to_string() }, None)
        .await
    {
        Ok(Some(user)) => {
            let pwd_correct =
                argon2::verify_encoded(user.password.as_str(), &req_body.password.as_bytes())
                    .unwrap();
            log::debug!("pwd_correct: {pwd_correct}");
            if pwd_correct {
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
            } else {
                return HttpResponse::InternalServerError().body("Bad password");
            }
        }
        Ok(None) => HttpResponse::NotFound().body(format!(
            "No user found with email {}",
            &req_body.email.to_string()
        )),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    };
    // let matching = verify(&user.hash, &auth_data.password);
    HttpResponse::NoContent().into()
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
pub struct AuthenticationInfo {
    // TODO later: Was an enum
    user: User,
    api_key: String,
}

/*impl AuthenticationInfo {
    pub fn new(api_key: String, user: User)->AuthenticationInfo{
        AuthenticationInfo::ApiKey { key: api_key }
    }
}*/

#[derive(Clone, Debug)]
pub struct AppState {
    pub mongo_db: mongodb::Client,
    /// Temporary method of implementing admin user
    pub admin_user: Option<ObjectId>,
}

use actix_web::{FromRequest, HttpMessage, HttpRequest};

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
            None => {
                log::error!("Empty Authenticated");
                Err(Error::AuthenticationError)
            }
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
        _identity: Option<actix_identity::Identity>,
        req: &ServiceRequest,
    ) -> Result<Option<AuthenticationInfo>, Error> {
        let auth = req.headers().get("Authorization");
        match auth {
            Some(_) => {
                let split: Vec<&str> = auth.unwrap().to_str().unwrap().split("Bearer ").collect();

                let mut token: String = "".to_string();
                for element in split {
                    if !element.is_empty() {
                        token = element.trim().to_string();
                    }
                }
                match check_jwt(token) {
                    Ok(token_claims) => {
                        // Function returned Ok, do something with token_claims
                        //log::debug!("Token claims: {:?}", token_claims);
                        let req_user = self.get_user_info(token_claims.user_id).await?;
                        //log::debug!("req_user: {req_user:?}");
                        let user = AuthenticationInfo {
                            user: req_user,
                            api_key: "".to_string(),
                        };
                        //log::debug!("user: {user:?}");
                        Ok(Some(user))
                    }
                    Err((status, error_response)) => {
                        // Function returned an error, handle the error
                        println!("Status code: {:?}", status);
                        println!("Error response: {:?}", error_response);
                        Err(Error::AuthorizationError)
                    }
                }
            }
            None => {
                log::info!("Spurious request for: {}", req.path());
                Err(Error::AuthenticationError)
            }
        }
        // match identity {
        //     Some(identity) => {
        //         let user_id =
        //             ObjectId::from_str(&identity.id()
        //                 .map_err(|_| Error::AuthenticationError)?)
        //                     .map_err(|_| Error::AuthenticationError)?;

        //         let req_user = self.get_user_info(&user_id).await?;
        //         Ok(Some(AuthenticationInfo::User(req_user)))
        //     }
        //     None => Ok(None),
        // }
    }

    async fn get_user_info(&self, user_id: String) -> Result<User, Error> {
        let collection: Collection<users::User> = self
            .mongo_db
            .database(DB_NAME)
            .collection(users::REPOSITORY_NAME);
        let user_object_id = ObjectId::parse_str(user_id).unwrap();
        match collection
            .find_one(doc! { "_id": &user_object_id }, None)
            .await
        {
            Ok(Some(user)) => Ok(User {
                _id: user._id,
                first_name: user.first_name,
                last_name: user.last_name,
                email: user.email,
                role: user.role,
                org_id: user.org_id,
                password: "".to_string(),
                //created: user.created,
            }),
            Ok(None) => Err(DatabaseError("User not found".to_string())),
            Err(err) => {
                log::error!("get_user_info err: {err}");
                Err(DatabaseError(err.to_string()))
            }
        }
        // let mut conn = self.pg.acquire().await?;
        // get_user_info(&mut conn, user_id, self.admin_user.as_ref()).await
    }
}
