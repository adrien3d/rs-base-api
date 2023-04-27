pub static SECRET_KEY: Lazy<String> =
    Lazy::new(|| std::env::var("SECRET_KEY").unwrap_or_else(|_| "0123".repeat(16)));

const SALT: &[u8] = b"supersecuresalt";

pub fn hash_password(password: &str) -> String {
    let context = argon2::Argon2::new_with_secret(SECRET_KEY.as_bytes(), argon2::Algorithm::Argon2id, argon2::Version::V0x13, argon2::Params::default());
    /*let config = argon2::Config {
        secret: SECRET_KEY.as_bytes(),
        ..Default::default()
    };
    argon2::hash_encoded(password.as_bytes(), SALT, &config).map_err(|err| {
        dbg!(err);
        ServiceError::InternalServerError
    })*/
    let hashed_pwd : &mut [u8] = &mut [];
    argon2::Argon2::hash_password_into(&context.unwrap(), password.as_bytes(), SALT, hashed_pwd);
    return String::from_utf8(Vec::from(hashed_pwd)).unwrap()
}