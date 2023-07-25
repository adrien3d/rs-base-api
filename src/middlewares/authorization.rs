use crate::controllers::authentication::{AppState, AuthenticationInfo};

use std::rc::Rc;

use actix_identity::IdentityExt;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::{
    future::{ready, LocalBoxFuture, Ready},
    FutureExt,
};

pub struct AuthenticateMiddlewareFactory {
    auth_data: Rc<AppState>,
}

impl AuthenticateMiddlewareFactory {
    pub fn new(auth_data: AppState) -> Self {
        AuthenticateMiddlewareFactory {
            auth_data: Rc::new(auth_data),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthenticateMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    type Response = ServiceResponse<B>;

    type Error = Error;

    type Transform = AuthenticateMiddleware<S>;

    type InitError = ();

    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticateMiddleware {
            auth_data: self.auth_data.clone(),
            service: Rc::new(service),
        }))
    }
}

pub struct AuthenticateMiddleware<S> {
    auth_data: Rc<AppState>,
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthenticateMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);
        let auth_data = self.auth_data.clone();

        async move {
            let id = req.get_identity().ok();
            let auth = auth_data.authenticate(id, &req).await?;
            if let Some(auth) = auth {
                req.extensions_mut()
                    .insert::<Rc<AuthenticationInfo>>(Rc::new(auth));
            }

            let res = srv.call(req).await?;

            Ok(res)
        }
        .boxed_local()
    }
}

// // There are two steps in middleware processing.
// // 1. Middleware initialization, middleware factory gets called with
// //    next service in chain as parameter.
// // 2. Middleware's call method gets called with normal request.
// pub struct Authorization;

// // Middleware factory is `Transform` trait
// // `S` - type of the next service
// // `B` - type of response's body
// impl<S, B> Transform<S, ServiceRequest> for Authorization
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
//     S::Future: 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<B>;
//     type Error = Error;
//     type InitError = ();
//     type Transform = AuthorizationMiddleware<S>;
//     type Future = Ready<Result<Self::Transform, Self::InitError>>;

//     fn new_transform(&self, service: S) -> Self::Future {
//         ready(Ok(AuthorizationMiddleware { service }))
//     }
// }

// pub struct AuthorizationMiddleware<S> {
//     service: S,
// }

// impl<S, B> Service<ServiceRequest> for AuthorizationMiddleware<S>
// where
//     S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
//     S::Future: 'static,
//     B: 'static,
// {
//     type Response = ServiceResponse<B>;
//     type Error = Error;
//     type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

//     forward_ready!(service);

//     fn call(&self, req: ServiceRequest) -> Self::Future {
//         println!("Hi from start. You requested: {}", req.path());
//         let auth = req.headers().get("Authorization");
//         match auth {
//             Some(_) => {
//                 let split: Vec<&str> = auth.unwrap().to_str().unwrap().split("Bearer").collect();
//                 let token = split[1].trim().to_string();
//                 match check_jwt(token) {
//                     Ok(token_claims) => {
//                         // Function returned Ok, do something with token_claims
//                         log::debug!("Token claims: {:?}", token_claims);
//                     }
//                     Err((status, error_response)) => {
//                         // Function returned an error, handle the error
//                         println!("Status code: {:?}", status);
//                         println!("Error response: {:?}", error_response);
//                     }
//                 }
//             }
//             None => {
//                 log::info!("Spurious request for: {}", req.path())
//             }
//         }

//         let fut = self.service.call(req);

//         Box::pin(async move {
//             let res = fut.await?;

//             println!("Hi from response");
//             Ok(res)
//         })
//     }
// }

// fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
//     let auth = req.headers().get("Authorization");
//     match auth {
//         Some(_) => {
//             let split: Vec<&str> = auth.unwrap().to_str().unwrap().split("Bearer").collect();
//             let token = split[1].trim();
//             let secret_key = "supersecret".as_bytes();
//             match decode::<TokenClaims>(
//                 &token.to_string(),
//                 &DecodingKey::from_secret(secret_key.as_ref()),
//                 &Validation::new(Algorithm::HS256),
//             ) {
//                 Ok(_token) => {
//                     let user_id = _token.claims.user_id;
//                     let role = _token.claims.role;
//                     ok(AuthorizationMiddleware { user_id, role })
//                 }
//                 Err(_e) => err(ErrorUnauthorized(_e)),
//             }
//         }
//         None => err(ErrorUnauthorized("Blocked")),
//     }
// }
