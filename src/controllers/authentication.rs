use actix_web::{post, HttpResponse, Responder, web};
use mongodb::{bson::doc, Client, Collection};
use crate::models::users;

#[post("/auth")]
pub(crate) async fn authentication(client: web::Data<Client>, req_body: web::Json<users::AuthReq>) -> impl Responder {
    //println!("{} {}", req_body.email, req_body.password);
    let email = req_body.email.to_string();
    let collection: Collection<users::User> = client.database(crate::controllers::users::DB_NAME).collection(users::REPOSITORY_NAME);
    match collection
        .find_one(doc! { "email": &email }, None)
        .await
    {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => {
            return HttpResponse::NotFound().body(format!("No user found with email {email}"));
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    };
    // let matching = verify(&user.hash, &auth_data.password);
    HttpResponse::Ok().json(req_body)
}