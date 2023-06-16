/*use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpResponse};
use futures::future::{ok, Ready};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct AuthorizationMiddleware;

impl<S, B> Transform<S> for AuthorizationMiddleware
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthorizationMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let middleware_service = AuthorizationMiddlewareService { service };
        ok(middleware_service)
    }
}

pub struct AuthorizationMiddlewareService<S> {
    service: S,
}

impl<S, B> Service for AuthorizationMiddlewareService<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn futures::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        // Implement your authorization logic here

        // For example, you can check for a specific header
        match req.headers().get("Authorization") {
            Some(header_value) => {
                // Perform authorization logic here
                // ...
                // Call the inner service
                let fut = self.service.call(req);

                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res)
                })
            }
            None => {
                // Unauthorized request
                let res = HttpResponse::Unauthorized().body("Unauthorized");
                Box::pin(async { Ok(req.into_response(res.into_body())) })
            }
        }
    }
}
*/