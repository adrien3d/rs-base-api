#[post("/auth")]
async fn authentication(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}