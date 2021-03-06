use axum::{
    body::{boxed, Full},
    http::{header, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

/// Send static file
/// TODO
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches("/").to_string();
    StaticFile(path)
}

#[derive(RustEmbed)]
#[folder = "dist/"]
struct Asset;
pub struct StaticFile<T>(pub T);

static CACHE_CONTROL_TIME: &str = "max-age=604800";

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();
        match Asset::get(path.as_str()) {
            Some(content) => {
                let body = boxed(Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .header(header::CACHE_CONTROL, CACHE_CONTROL_TIME)
                    .body(body)
                    .unwrap()
            }
            None => {
                // Returning index as default because we are bundling Single-page application
                let data = Asset::get("index.html").unwrap().data;
                let body = boxed(Full::from(data));
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .header(header::CACHE_CONTROL, CACHE_CONTROL_TIME)
                    .body(body)
                    .unwrap()
            }
        }
    }
}
