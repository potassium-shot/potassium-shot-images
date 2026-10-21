use std::{borrow::Cow, io::BufWriter};

use axum::{body::Body, http::header, response::Response, routing::get};
use axum_anyhow::{ApiResult, ResultExt};
use image::DynamicImage;
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

#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct GetImageQueryParams {
    download: bool,
    file_name: Option<String>,
    max_width: Option<u32>,
    max_height: Option<u32>,
}

async fn get_image(
    axum::extract::Path(path): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<GetImageQueryParams>,
) -> ApiResult<Response> {
    let full_path = env::IMAGES_DIRECTORY.join(path);
    let mut file = tokio::fs::File::open(&full_path)
        .await
        .context_not_found("Finding the specified image file")?;
    let file_len = file
        .metadata()
        .await
        .context_internal("Checking the image file length")?
        .len();

    let mut first_bytes = vec![0_u8; 64];
    file.read_exact(&mut first_bytes)
        .await
        .context_internal("Reading the start of the image file")?;
    let guessed_format =
        image::guess_format(&first_bytes).context_internal("Guessing the format of the image")?;
    file.seek(std::io::SeekFrom::Start(0))
        .await
        .context_internal("Preparing image file for reading")?;

    let file_name = params
        .file_name
        .as_ref()
        .map(|s| Cow::Borrowed(s.as_str()))
        .unwrap_or_else(|| {
            full_path
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or(match guessed_format.extensions_str().first() {
                    Some(ext) => Cow::Owned(format!("image.{}", ext)),
                    None => Cow::Borrowed("image"),
                })
        });

    let disposition = if params.download {
        "attachement"
    } else {
        "inline"
    };

    let response = Response::builder()
        .header(header::CONTENT_TYPE, guessed_format.to_mime_type())
        .header(
            header::CONTENT_DISPOSITION,
            format!("{}; filename=\"{}\"", disposition, file_name),
        );

    if params.require_treatment() {
        let mut all_bytes = Vec::new();
        file.read_to_end(&mut all_bytes)
            .await
            .context_internal("Reading image file before treatment")?;

        let image = image::load_from_memory_with_format(&all_bytes, guessed_format)
            .context_internal("Decoding image data")?;
        let image = params.treat_image(image);

        all_bytes.clear();

        let encoder = image::codecs::png::PngEncoder::new(BufWriter::new(&mut all_bytes));
        image.write_with_encoder(encoder)?;

        Ok(response
            .header(header::CONTENT_LENGTH, all_bytes.len().to_string())
            .body(Body::from(all_bytes))
            .unwrap())
    } else {
        Ok(response
            .header(header::CONTENT_LENGTH, file_len.to_string())
            .body(Body::from_stream(ReaderStream::with_capacity(file, 65536)))
            .unwrap())
    }
}

impl GetImageQueryParams {
    fn require_treatment(&self) -> bool {
        self.max_width.is_some() || self.max_height.is_some()
    }

    fn treat_image(&self, image: DynamicImage) -> DynamicImage {
        image.thumbnail(
            self.max_width.unwrap_or(u32::MAX),
            self.max_height.unwrap_or(u32::MAX),
        )
    }
}
