use std::time::SystemTime;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    headers::Cookie,
    headers::{authorization::Bearer, Authorization},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, SqlitePool};

static KEY: &str = "yurayuraringdong~";

lazy_static! {
    static ref ENCRYPT_KEY: EncodingKey = EncodingKey::from_secret(KEY.as_bytes());
    static ref DECRYPT_KEY: DecodingKey = DecodingKey::from_secret(KEY.as_bytes());
    static ref DEFAULT_HEADER: Header = Header::default();
    static ref VALIDATION: Validation = Validation::default();
    static ref SALT: SaltString = SaltString::new("AAAABBBBCCCCDDDD").unwrap();
}

#[derive(Serialize)]
pub struct Token {
    token: String,
}

#[derive(Deserialize, FromRow)]
pub struct QueryUser {
    username: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPassword {
    old_password: String,
    new_password: String,
}

#[derive(Deserialize, Serialize)]
pub struct Claim {
    sub: String,
    username: String,
    exp: u64,
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    DatabaseError,
}

fn get_unix_timestamp() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs(),
        Err(_) => 0,
    }
}

/// Check password and hash
fn check_hash(password: &String, hash: &String) -> bool {
    let hash_db = match PasswordHash::new(&hash) {
        Ok(h) => h,
        _ => return false,
    };
    match Argon2::default().verify_password(password.as_bytes(), &hash_db) {
        Ok(_) => true,
        _ => false,
    }
}

/// Generate hash
fn gen_hash(password: &String) -> Option<String> {
    Some(
        Argon2::default()
            .hash_password(password.as_bytes(), SALT.as_ref())
            .ok()?
            .to_string(),
    )
}

/// Login authorization
pub async fn authorize(
    Json(payload): Json<QueryUser>,
    Extension(pool): Extension<SqlitePool>,
) -> Result<Response, AuthError> {
    // Check if the user sent the credentials
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    // Get password hash in database
    let result = sqlx::query!(
        "SELECT password FROM user WHERE username = ?",
        payload.username
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| AuthError::DatabaseError)?;
    // Verify password hash
    match result.password {
        Some(p) if check_hash(&payload.password, &p) => (),
        _ => return Err(AuthError::WrongCredentials),
    }
    // Create the authorization token
    let claims = Claim {
        sub: "file".to_owned(),
        username: payload.username,
        exp: get_unix_timestamp() + 60 * 60 * 24,
    };
    let token =
        encode(&Header::default(), &claims, &ENCRYPT_KEY).map_err(|_| AuthError::TokenCreation)?;
    // Add token to cookies
    let cookie = format!("Authorization=Bearer {}; ", &token);
    let mut response = Json(Token { token }).into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie.parse().unwrap());
    Ok(response)
}

/// Register
pub async fn register(
    Json(payload): Json<QueryUser>,
    Extension(pool): Extension<SqlitePool>,
) -> Result<(), AuthError> {
    // Hash password and store it into database
    let password_hash = gen_hash(&payload.password).ok_or(AuthError::TokenCreation)?;
    sqlx::query!(
        "INSERT INTO user (username, password) VALUES (?, ?)",
        payload.username,
        password_hash
    )
    .execute(&pool)
    .await
    .map_err(|_| AuthError::DatabaseError)?;
    Ok(())
}

/// Reset password with old and new password
pub async fn reset_password(
    Json(payload): Json<ResetPassword>,
    claim: Claim,
    Extension(pool): Extension<SqlitePool>,
) -> Result<StatusCode, AuthError> {
    let result = sqlx::query!(
        "SELECT password FROM user WHERE username = ?",
        claim.username
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| AuthError::DatabaseError)?;
    let password_db = result.password.ok_or(AuthError::DatabaseError)?;
    // Check user-input old password is correct
    if check_hash(&payload.old_password, &password_db) {
        // Only if the old password is correct, we can modify password in database
        let new_password_hash = gen_hash(&payload.new_password).ok_or(AuthError::TokenCreation)?;
        sqlx::query!(
            "UPDATE user SET password = ? WHERE username = ?",
            new_password_hash,
            claim.username
        )
        .execute(&pool)
        .await
        .map_err(|_| AuthError::DatabaseError)?;
        Ok(StatusCode::OK)
    } else {
        Err(AuthError::WrongCredentials)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, "Database query error"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

#[async_trait]
impl<B> FromRequest<B> for Claim
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let bearer = if let Ok(TypedHeader(Authorization(bearer))) =
            TypedHeader::<Authorization<Bearer>>::from_request(req).await
        {
            bearer.token().to_string()
        } else {
            // If header don't have authorizaiton header, attempt to find it in cookie
            let cookie = Option::<TypedHeader<Cookie>>::from_request(req)
                .await
                .map_err(|_| AuthError::MissingCredentials)?;
            let auth_cookie = cookie
                .as_ref()
                .and_then(|cookie| cookie.get("Authorization"))
                .ok_or(AuthError::MissingCredentials)?;
            auth_cookie
                .strip_prefix("Bearer ")
                .ok_or(AuthError::InvalidToken)?
                .to_string()
        };

        // Decode the user data
        let token_data = decode::<Claim>(&bearer, &DECRYPT_KEY, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hash() {
        let password = b"PASSWORD";
        let password_hash = Argon2::default().hash_password(password, SALT.as_ref());
        assert!(password_hash.is_ok(), "{:#?}", password_hash);
    }
}
