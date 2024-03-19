use actix_web::{
    test::{call_and_read_body, call_and_read_body_json, init_service, TestRequest},
    web::Bytes,
};
use mongodb::Client;

use crate::{
    controllers::users::{create_user, get_user_by_email},
    models::users,
};

use super::*;

#[actix_web::test]
#[ignore = "requires MongoDB instance running"]
async fn test() {
    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());

    let client = Client::with_uri_str(uri).await.expect("failed to connect");

    // Clear any data currently in the users collection.
    client
        .database(&MONGODB_URI)
        .collection::<User>(users::REPOSITORY_NAME)
        .drop(None)
        .await
        .expect("drop collection should succeed");

    let app = init_service(
        App::new()
            .app_data(web::Data::new(client))
            .service(create_user)
            .service(get_user_by_email),
    )
    .await;

    let user = User {
        _id: ObjectId::new(),
        first_name: "Jane".into(),
        last_name: "Doe".into(),
        email: "example@example.com".into(),
        role: "".to_string(),
        org_id: Some(ObjectId::new()),
        password: "".to_string(),
    };

    let req = TestRequest::post()
        .uri("/add_user")
        .set_form(&user)
        .to_request();

    let response = call_and_read_body(&app, req).await;
    assert_eq!(response, Bytes::from_static(b"user added"));

    let req = TestRequest::get()
        .uri(&format!("/get_user/{}", &user.email))
        .to_request();

    let response: User = call_and_read_body_json(&app, req).await;
    assert_eq!(response, user);
}
