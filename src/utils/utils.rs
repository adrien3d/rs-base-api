use argon2::{self, Config};

pub fn hash_password(password: &str) -> String {
    let salt = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(16));
    let config = Config::default();
    argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &config).unwrap()
}
