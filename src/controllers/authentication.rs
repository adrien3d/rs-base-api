use crate::models::users;
use actix_web::{
    http::StatusCode,
    post,
    web::{self, Json},
    HttpResponse, Responder,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mongodb::{bson::doc, Client, Collection};
use serde::{Deserialize, Serialize};

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

    let collection: Collection<users::User> = client
        .database(crate::controllers::users::DB_NAME)
        .collection(users::REPOSITORY_NAME);
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
