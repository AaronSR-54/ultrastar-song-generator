use std::fs::File;
use std::path::Path;
use std::time::Duration;

use image::{DynamicImage, ImageEncoder};
use reqwest::Client;
use usdx_ports::CancelFlag;

use crate::metadata::VideoMetadata;

pub const COVER_SIZE: u32 = 800;
const MAX_THUMBNAIL_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum CoverError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid image: {0}")]
    Image(#[from] image::ImageError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("the video has no thumbnail")]
    MissingThumbnail,
    #[error("thumbnail is larger than the {MAX_THUMBNAIL_BYTES} byte limit")]
    TooLarge,
    #[error("cover download was cancelled")]
    Cancelled,
}

/// Crops an image to a centred square and re-encodes it as JPEG.
pub fn save_square_cover(input: &Path, output: &Path, size: u32) -> Result<(), CoverError> {
    let image = image::open(input)?;
    write_square(image, output, size)
}

/// Same as [`save_square_cover`] but from raw image bytes.
pub fn save_square_bytes(bytes: &[u8], output: &Path, size: u32) -> Result<(), CoverError> {
    let image = image::load_from_memory(bytes)?;
    write_square(image, output, size)
}

fn write_square(image: DynamicImage, output: &Path, size: u32) -> Result<(), CoverError> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let square = image.resize_to_fill(size, size, image::imageops::FilterType::Lanczos3);
    let rgb = square.to_rgb8();
    let mut file = File::create(output)?;
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 90);
    encoder.write_image(
        rgb.as_raw(),
        rgb.width(),
        rgb.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(())
}

/// Downloads the thumbnail referenced by the metadata and stores a square cover.
pub async fn download_cover(
    metadata: &VideoMetadata,
    output: &Path,
    cancel: Option<CancelFlag>,
) -> Result<(), CoverError> {
    let url = metadata
        .thumbnail_url
        .as_ref()
        .ok_or(CoverError::MissingThumbnail)?;
    if cancel.as_ref().is_some_and(CancelFlag::is_cancelled) {
        return Err(CoverError::Cancelled);
    }

    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let response = client.get(url).send().await?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_THUMBNAIL_BYTES as u64)
    {
        return Err(CoverError::TooLarge);
    }
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_THUMBNAIL_BYTES {
        return Err(CoverError::TooLarge);
    }
    save_square_bytes(&bytes, output, COVER_SIZE)
}

/// Replaces the cover with a user-selected image, validating and cropping it.
pub fn replace_cover(local: &Path, output: &Path) -> Result<(), CoverError> {
    save_square_cover(local, output, COVER_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn write_test_png(path: &Path, width: u32, height: u32) {
        let image = RgbImage::from_pixel(width, height, Rgb([120, 30, 200]));
        image.save(path).unwrap();
    }

    #[test]
    fn crops_to_a_centred_square() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("wide.png");
        let output = directory.path().join("cover.jpg");
        write_test_png(&input, 1000, 500);

        save_square_cover(&input, &output, 800).unwrap();
        let result = image::open(&output).unwrap();
        assert_eq!(result.width(), 800);
        assert_eq!(result.height(), 800);
    }

    #[test]
    fn accepts_encoded_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("source.png");
        write_test_png(&input, 300, 300);
        let bytes = std::fs::read(&input).unwrap();
        let output = directory.path().join("cover.jpg");
        save_square_bytes(&bytes, &output, 400).unwrap();
        let result = image::open(&output).unwrap();
        assert_eq!(result.width(), 400);
        assert_eq!(result.height(), 400);
    }

    #[test]
    fn rejects_invalid_images() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("not-an-image.jpg");
        std::fs::write(&input, b"not an image").unwrap();
        let output = directory.path().join("cover.jpg");
        assert!(matches!(
            replace_cover(&input, &output),
            Err(CoverError::Image(_))
        ));
    }
}
