use std::fs::read;

use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

use crate::{
    file::{concat_path_str, is_traversal, File, FileError},
    user::Claim,
};

#[derive(Deserialize)]
pub struct AddShareArgs {
    path: String,
    password: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryShareArgs {
    url: String,
    file_path: String,
    password: Option<String>,
    download: Option<bool>,
}

/// All field is optional,
/// because all field is null by default in database
#[derive(Serialize)]
pub struct ShareIndex {
    path: Option<String>,
    url: Option<String>,
    /// Password is optional in creation
    password: Option<String>,
}

/// Add share file/folder, return url
pub async fn add_share_file(
    Query(args): Query<AddShareArgs>,
    Extension(db): Extension<SqlitePool>,
    _: Claim,
) -> Result<impl IntoResponse, FileError> {
    let path = concat_path_str(&args.path);
    if is_traversal(&path) {
        return Err(FileError::PathError);
    }
    let mut counter = 0; // Set a counter to limit rng generate frequency
    let url = loop {
        // Generate random url and ensure it is unique
        let random = rand::random::<u32>().to_string();
        let result = sqlx::query!("SELECT url FROM share where url = ?", random)
            .fetch_all(&db)
            .await?;
        if result.len() == 0 {
            break random;
        }
        counter += 1;
        if counter > 10 {
            return Err(FileError::ServerError);
        }
    };
    sqlx::query!(
        "INSERT INTO share (path, url, password) VALUES (?, ?, ?)",
        args.path,
        url,
        args.password
    )
    .execute(&db)
    .await?;
    Ok(Json(json!({ "url": url })))
}

/// Delete share
pub async fn delete_share(
    Query(args): Query<AddShareArgs>,
    Extension(db): Extension<SqlitePool>,
    _: Claim,
) -> Result<StatusCode, FileError> {
    sqlx::query!("DELETE FROM share WHERE path = ?", args.path)
        .execute(&db)
        .await?;
    Ok(StatusCode::OK)
}

/// Get share file/folder
pub async fn get_share_file(
    Query(args): Query<QueryShareArgs>,
    Extension(db): Extension<SqlitePool>,
) -> Result<Response, FileError> {
    let result = sqlx::query!("SELECT * FROM share WHERE url = ?", args.url)
        .fetch_one(&db)
        .await?;
    if args.password != result.password {
        return Err(FileError::ContentError);
    }
    let path = result.path.ok_or(FileError::PathError)?;
    let mut path = concat_path_str(&path);
    // When share file is single file, don't concat file_path
    if !path.is_file() {
        path.push(args.file_path);
    }
    // Because of the user-input `file_path`, we should check `path` again
    if is_traversal(&path) {
        return Err(FileError::PathError);
    }
    if path.is_dir() {
        Ok(Json(File::read_dir(&path)?).into_response())
    } else {
        if args.download == Some(true) {
            Ok(read(&path)?.into_response())
        } else {
            Ok(Json(File::new(&path)?).into_response())
        }
    }
}

/// Get all share file
pub async fn get_share_index(
    Extension(db): Extension<SqlitePool>,
    _: Claim,
) -> Result<Json<Vec<ShareIndex>>, FileError> {
    let result = sqlx::query_as!(ShareIndex, "SELECT path, url, password FROM share")
        .fetch_all(&db)
        .await?;
    Ok(Json(result))
}
