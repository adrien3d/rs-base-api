use actix_web::{body::BoxBody, http::StatusCode, HttpResponse};
use envoption::EnvOptionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Authentication failure")]
    Authentication,

    #[error("Unauthorized")]
    Authorization,

    //#[error("Password hasher error: {0}")]
    //PasswordHasherError(String),
    #[error("Environment variable error: {0}")]
    EnvOption(String),

    // #[error("DB Error")]
    // SqlError(#[from] sqlx::error::Error),
    #[error("DB error")]
    Database(String),
}

impl<T: std::error::Error> From<EnvOptionError<T>> for Error {
    fn from(e: EnvOptionError<T>) -> Self {
        Self::EnvOption(e.to_string())
    }
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Error::Authentication => StatusCode::UNAUTHORIZED,
            Error::Authorization => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
