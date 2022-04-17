use std::path::PathBuf;

use axum::{
    body::Bytes,
    extract::{Multipart, Query},
    http::StatusCode,
    Json,
};
use tokio::fs::{remove_dir, remove_file, rename, write};

use crate::{
    file::{concat_path_str, is_traversal, File, FileError, Path, QueryArgs, RenameArgs},
    user::Claim,
    CONFIG,
};

/// Delete file
pub async fn delete_file(Path(path): Path, _: Claim) -> Result<StatusCode, FileError> {
    if path.is_dir() {
        remove_dir(path).await?;
    } else {
        remove_file(path).await?;
    }
    Ok(StatusCode::OK)
}

/// Rename file
pub async fn rename_file(
    Query(args): Query<RenameArgs>,
    _: Claim,
) -> Result<StatusCode, FileError> {
    let from = concat_path_str(&args.from);
    let to = concat_path_str(&args.to);
    if is_traversal(&from) || is_traversal(&to) {
        return Err(FileError::PathError);
    }
    rename(from, to).await?;
    Ok(StatusCode::OK)
}

/// Using multipart to accept upload file
pub async fn upload_file(mut multipart: Multipart, _: Claim) -> Result<StatusCode, FileError> {
    // New file `data` will be store in `FOLDER + path + name`
    let mut path: Option<String> = None;
    let mut data: Option<Bytes> = None;
    let mut name: Option<String> = None;
    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("path") => path = Some(field.text().await?),
            Some("file") => {
                name = Some(
                    field
                        .file_name()
                        .ok_or(FileError::ContentError)?
                        .to_string(),
                );
                data = Some(field.bytes().await?);
                break;
            }
            _ => return Err(FileError::ContentError),
        }
    }
    let path = path.ok_or(FileError::ContentError)?;
    let data = data.ok_or(FileError::ContentError)?;
    let name = name.ok_or(FileError::ContentError)?;
    let mut path = concat_path_str(&path);
    path.push(&name);
    if is_traversal(&path) {
        return Err(FileError::PathError);
    }
    // Don't write to a exist file
    if path.is_file() {
        return Err(FileError::PathError);
    }
    write(path, data).await?;
    Ok(StatusCode::OK)
}

/// Search file based on name
pub async fn search_file(
    Query(args): Query<QueryArgs>,
    _: Claim,
) -> Result<Json<Vec<File>>, FileError> {
    let mut search_folders: Vec<PathBuf> = vec![PathBuf::from(CONFIG.folder_path.clone())];
    let mut files = vec![];
    // Iter all folders to find matched file and folder
    while search_folders.len() != 0 {
        let folder = search_folders.pop().ok_or(FileError::PathError)?;
        for f in File::read_dir(&folder)? {
            // Push folder into search list
            if f.type_ == "folder" {
                // Concat path
                let mut new_folder = folder.clone();
                new_folder.push(&f.name);
                search_folders.push(new_folder);
            }
            if f.name.contains(&args.name) {
                files.push(f.absolute_path(&folder).ok_or(FileError::PathError)?);
            }
        }
    }
    Ok(Json(files))
}
