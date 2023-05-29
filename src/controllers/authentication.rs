use actix_web::{post, HttpResponse, Responder, web};
use jsonwebtoken::{encode, EncodingKey, Header};
use mongodb::{bson::doc, Client, Collection};
use serde::{Serialize, Deserialize};
use crate::models::users;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}

#[post("/auth")]
pub(crate) async fn authentication(client: web::Data<Client>, req_body: web::Json<users::AuthReq>) -> impl Responder {
    //println!("{} {}", req_body.email, req_body.password);

    let secret = "supersecret";//std::env::var("RSA_KEY");
    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;

    let collection: Collection<users::User> = client.database(crate::controllers::users::DB_NAME).collection(users::REPOSITORY_NAME);
    match collection
        .find_one(doc! { "email": &req_body.email.to_string() }, None)
        .await
    {
        Ok(Some(user)) => {
            let claims: TokenClaims = TokenClaims {
                sub: user._id.to_string(),
                exp,
                iat,
            };

            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(secret.as_ref()),
            )
            .unwrap();

            return HttpResponse::Ok().json(token)
        },
        Ok(None) => {
            return HttpResponse::NotFound().body(format!("No user found with email {}", &req_body.email.to_string()));
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    };
    // let matching = verify(&user.hash, &auth_data.password);
    HttpResponse::Ok().json(req_body)
}