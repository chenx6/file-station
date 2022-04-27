pub mod file;
pub mod folder;
pub mod share;

use std::fs::{canonicalize, metadata, read_dir};
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use axum::extract::multipart::MultipartError;
use axum::extract::{FromRequest, Path, RequestParts};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Json};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::CONFIG;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    name: String,
    size: u64,
    #[serde(rename = "type")]
    type_: String,
    last_modified_time: u64,
    /// Indicate the absolute path of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    absolute_path: Option<String>,
}

impl File {
    /// Get the file infomation from `path`
    fn new(path: &PathBuf) -> Result<File, FileError> {
        let meta = metadata(path)?;
        let name = match path.file_name() {
            Some(s) => match s.to_str() {
                Some(s) => s.to_string(),
                None => return Err(FileError::PathError),
            },
            None => return Err(FileError::PathError),
        };
        let last_modified_time = match meta.modified() {
            Ok(m) => match m.duration_since(SystemTime::UNIX_EPOCH) {
                Ok(m) => m.as_secs(),
                Err(_) => 0,
            },
            Err(_) => 0,
        };
        Ok(File {
            name,
            size: meta.len(),
            type_: if meta.is_dir() { "folder" } else { "file" }.into(),
            last_modified_time,
            absolute_path: None,
        })
    }

    /// Add "absolute" folder `path` to file
    fn absolute_path(mut self, path: &PathBuf) -> Option<Self> {
        let mut abs_path = if path.starts_with(&CONFIG.folder_path) {
            path.strip_prefix(&CONFIG.folder_path).ok()?.to_path_buf()
        } else {
            return None;
        };
        abs_path.push(&self.name);
        self.absolute_path = Some(abs_path.to_str()?.to_string());
        Some(self)
    }

    /// Get the information of file in the `path` folder
    fn read_dir(path: &PathBuf) -> Result<Vec<File>, FileError> {
        let files: Vec<_> = read_dir(path)?
            .filter_map(|rd| rd.ok())
            .filter_map(|v| File::new(&v.path()).ok())
            .collect();
        Ok(files)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FileError {
    #[error("Io Error")]
    IoError(#[from] io::Error),
    #[error("Path Error")]
    PathError,
    #[error("Upload Error")]
    UploadError(#[from] MultipartError),
    #[error("Content error")]
    ContentError,
    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Server error")]
    ServerError,
}

impl IntoResponse for FileError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": self.to_string()
            })),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
pub struct QueryArgs {
    name: String,
}

#[derive(Deserialize)]
pub struct RenameArgs {
    to: String,
}

/// Path Extractor with check
pub struct CheckedPath(PathBuf);

#[async_trait]
impl<B> FromRequest<B> for CheckedPath
where
    B: Send,
{
    type Rejection = FileError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let path = req.uri().path();
        let path = if path.starts_with("/files/") {
            // use extractor to extract relative path of `/files`
            let Path(p) = Path::<String>::from_request(req)
                .await
                .map_err(|_| FileError::PathError)?;
            p
        } else {
            // `/file` path contains relative path starts with '/'
            percent_decode_str(path)
                .decode_utf8()
                .map_err(|_| FileError::PathError)?
                .to_string()
        };
        // Concat and check path is valid
        let path = concat_path_str(&path);
        if is_traversal(&path) {
            return Err(FileError::PathError);
        }
        Ok(CheckedPath(path))
    }
}

/// Concat `s` to base path
fn concat_path_str(s: &String) -> PathBuf {
    let mut path = CONFIG.folder_path.clone();
    // AVOID ANTI-PATTEN `path.push` by triming '/' in the beginning of the path
    path.push(&PathBuf::from(s.trim_start_matches('/')));
    path
}

/// Detect path traversal
/// Because of the anti-patten path.push, we should use this function when we join paths
fn is_traversal(path: &PathBuf) -> bool {
    let abs_path = match canonicalize(path) {
        Ok(p) => p,
        Err(_) => match path.exists() {
            // Path not exist doesn't means it is illegal
            false => path.into(),
            true => return true,
        },
    };
    !abs_path.starts_with(&CONFIG.folder_path)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_traversal() {
        assert_eq!(is_traversal(&PathBuf::from("files/test_file")), false);
        assert_eq!(is_traversal(&PathBuf::from("src")), true);
        assert_eq!(is_traversal(&PathBuf::from("/etc/passwd")), true);
    }

    #[test]
    fn test_file_struct() {
        let file = File::new(&PathBuf::from("files/test_folder")).unwrap();
        let abs_path = canonicalize(&PathBuf::from("files")).unwrap();
        let file = file.absolute_path(&abs_path).unwrap();
        assert_eq!(file.absolute_path, Some("test_folder".to_string()));
    }
}
