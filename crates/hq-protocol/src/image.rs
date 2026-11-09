//! What the host takes as an image for a session (add-image-upload D3), so clients can check
//! before sending what the host checks again.

/// The largest image the host takes: 10 MiB.
pub const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// The image formats Claude Code and Codex read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageKind {
    /// The format the file's first bytes show, whatever its name or sender says.
    pub fn of(head: &[u8]) -> Option<ImageKind> {
        if head.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(ImageKind::Png)
        } else if head.starts_with(b"\xff\xd8\xff") {
            Some(ImageKind::Jpeg)
        } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
            Some(ImageKind::Gif)
        } else if head.len() >= 12 && head.starts_with(b"RIFF") && &head[8..12] == b"WEBP" {
            Some(ImageKind::Webp)
        } else {
            None
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpeg",
            ImageKind::Gif => "gif",
            ImageKind::Webp => "webp",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpg",
            ImageKind::Gif => "gif",
            ImageKind::Webp => "webp",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_are_read_from_the_first_bytes() {
        assert_eq!(
            ImageKind::of(b"\x89PNG\r\n\x1a\n\0\0"),
            Some(ImageKind::Png)
        );
        assert_eq!(
            ImageKind::of(b"\xff\xd8\xff\xe0\0\x10JFIF"),
            Some(ImageKind::Jpeg)
        );
        assert_eq!(ImageKind::of(b"GIF89a\x01\0"), Some(ImageKind::Gif));
        assert_eq!(
            ImageKind::of(b"RIFF\x24\0\0\0WEBPVP8 "),
            Some(ImageKind::Webp)
        );
        assert_eq!(ImageKind::of(b"%PDF-1.7\n"), None);
        assert_eq!(ImageKind::of(b"RIFF\x24\0\0\0WAVE"), None);
        assert_eq!(
            ImageKind::of(b"BM\x36\0"),
            None,
            "a bitmap is converted first"
        );
        assert_eq!(ImageKind::of(b""), None);
    }
}
