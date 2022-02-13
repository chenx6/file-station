use std::fs::create_dir;

use axum::{http::StatusCode, Json};

use crate::{
    file::{File, FileError, Path},
    user::Claim,
};

/// Get folder content based on args
pub async fn get_folder(Path(path): Path, _: Claim) -> Result<Json<Vec<File>>, FileError> {
    if !path.is_dir() {
        return Err(FileError::PathError);
    }
    Ok(Json(File::read_dir(&path)?))
}

/// Create folder
pub async fn create_folder(Path(path): Path, _: Claim) -> Result<StatusCode, FileError> {
    create_dir(path)?;
    Ok(StatusCode::OK)
}
