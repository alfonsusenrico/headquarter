//! Images for a session on another computer (add-image-upload D7): dropped image files and
//! images pasted from the clipboard, checked as the host checks them, and clipboard formats
//! the agents cannot read converted to PNG first.

use std::path::Path;

use hq_protocol::image::{ImageKind, MAX_IMAGE_BYTES};

pub const NOT_AN_IMAGE: &str = "Only images can be sent to another computer.";
pub const TOO_LARGE: &str = "The image is larger than 10 MB.";

/// A dropped file's bytes, when it is an image the host takes.
pub fn dropped(path: &Path) -> Result<Vec<u8>, &'static str> {
    let size = std::fs::metadata(path).map_err(|_| NOT_AN_IMAGE)?.len();
    if !path.is_file() {
        return Err(NOT_AN_IMAGE);
    }
    if size > MAX_IMAGE_BYTES {
        return Err(TOO_LARGE);
    }
    let bytes = std::fs::read(path).map_err(|_| NOT_AN_IMAGE)?;
    checked(bytes)
}

/// A clipboard image's bytes for the host: as they are when the agents read the format,
/// otherwise converted to PNG.
pub fn pasted(image: &gpui::Image) -> Result<Vec<u8>, &'static str> {
    if ImageKind::of(&image.bytes).is_some() {
        return checked(image.bytes.clone());
    }
    let decoded = image::load_from_memory(&image.bytes).map_err(|_| NOT_AN_IMAGE)?;
    let mut png = Vec::new();
    decoded
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|_| NOT_AN_IMAGE)?;
    checked(png)
}

fn checked(bytes: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    if ImageKind::of(&bytes).is_none() {
        return Err(NOT_AN_IMAGE);
    }
    if bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(TOO_LARGE);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clipboard(format: gpui::ImageFormat, bytes: Vec<u8>) -> gpui::Image {
        gpui::Image::from_bytes(format, bytes)
    }

    /// A 2×2 image in `format`, encoded by the `image` crate.
    fn encoded(format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut std::io::Cursor::new(&mut bytes), format)
            .unwrap();
        bytes
    }

    #[test]
    fn a_png_is_sent_as_it_is_and_a_bitmap_as_png() {
        let png = encoded(image::ImageFormat::Png);
        assert_eq!(
            pasted(&clipboard(gpui::ImageFormat::Png, png.clone())).unwrap(),
            png
        );
        let from_bitmap = pasted(&clipboard(
            gpui::ImageFormat::Bmp,
            encoded(image::ImageFormat::Bmp),
        ))
        .unwrap();
        assert_eq!(ImageKind::of(&from_bitmap), Some(ImageKind::Png));
        let from_tiff = pasted(&clipboard(
            gpui::ImageFormat::Tiff,
            encoded(image::ImageFormat::Tiff),
        ))
        .unwrap();
        assert_eq!(ImageKind::of(&from_tiff), Some(ImageKind::Png));
        assert_eq!(
            pasted(&clipboard(gpui::ImageFormat::Svg, b"<svg/>".to_vec())),
            Err(NOT_AN_IMAGE)
        );
    }

    #[test]
    fn a_dropped_file_must_be_an_image_the_host_takes() {
        let folder = std::env::temp_dir().join(format!("hq-app-drop-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let png = folder.join("shot.png");
        std::fs::write(&png, encoded(image::ImageFormat::Png)).unwrap();
        let pdf = folder.join("notes.png");
        std::fs::write(&pdf, b"%PDF-1.7\n").unwrap();
        let large = folder.join("large.png");
        let mut big = encoded(image::ImageFormat::Png);
        big.resize(MAX_IMAGE_BYTES as usize + 1, 0);
        std::fs::write(&large, big).unwrap();

        assert!(dropped(&png).is_ok());
        assert_eq!(
            dropped(&pdf),
            Err(NOT_AN_IMAGE),
            "by its bytes, not its name"
        );
        assert_eq!(dropped(&large), Err(TOO_LARGE));
        assert_eq!(dropped(&folder), Err(NOT_AN_IMAGE), "a folder");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
