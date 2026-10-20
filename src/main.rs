use std::borrow::Cow;

use axum::{body::Body, http::header, response::Response, routing::get};
use axum_anyhow::ApiResult;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

mod env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let bind_address = format!("{}:{}", *env::BIND_IP, *env::PORT);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    let router = axum::Router::new().route("/{*path}", get(get_image));

    tracing::info!("Listening on {}.", bind_address);

    tokio::select! {
        result = axum::serve(listener, router) => {
            tracing::info!("Stopped listening.");
            result?;
        },
        result = tokio::signal::ctrl_c() => {
            println!();
            tracing::info!("Stopped listening due to CTRL+C.");
            result?;
        },
    }

    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(default)]
struct GetImageQueryParams {
    download: bool,
}

impl Default for GetImageQueryParams {
    fn default() -> Self {
        Self { download: false }
    }
}

async fn get_image(
    axum::extract::Path(path): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<GetImageQueryParams>,
) -> ApiResult<Response> {
    let full_path = env::IMAGES_DIRECTORY.join(path);
    let mut file = tokio::fs::File::open(&full_path).await?;
    let file_len = file.metadata().await?.len();

    let mut first_bytes = vec![0_u8; 64];
    file.read_exact(&mut first_bytes).await?;
    let guessed_format = image::guess_format(&first_bytes)?;
    file.seek(std::io::SeekFrom::Start(0)).await?;

    let file_name = full_path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or(match guessed_format.extensions_str().first() {
            Some(ext) => Cow::Owned(format!("image.{}", ext)),
            None => Cow::Borrowed("image"),
        });

    let disposition = if params.download {
        "attachement"
    } else {
        "inline"
    };

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, guessed_format.to_mime_type())
        .header(
            header::CONTENT_DISPOSITION,
            format!("{}; filename=\"{}\"", disposition, file_name),
        )
        .header(header::CONTENT_LENGTH, file_len.to_string())
        .body(Body::from_stream(ReaderStream::with_capacity(file, 65536)))
        .unwrap())
}
