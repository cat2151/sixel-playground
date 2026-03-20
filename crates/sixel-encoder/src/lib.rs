//! `sixel-encoder` — encode RGBA image data as a sixel escape sequence for
//! display in sixel-capable terminal emulators.
//!
//! This crate is a thin wrapper around
//! [icy_sixel](https://crates.io/crates/icy_sixel) that exposes a
//! straightforward function for converting an RGBA pixel buffer into the
//! DCS escape string used for TUI sixel output.

use icy_sixel::{SixelError, SixelImage};

/// Encode an RGBA image buffer as a sixel escape sequence string.
///
/// The returned string begins with the DCS introducer (`\x1bP`) and ends with
/// the string terminator (`\x1b\\`).  Print it directly to stdout in a
/// supporting terminal to display the image inline.
///
/// # Arguments
/// * `rgba`   – flat RGBA pixel data (4 bytes per pixel, row-major)
/// * `width`  – image width in pixels
/// * `height` – image height in pixels
///
/// # Errors
/// Returns a [`SixelError`] if encoding fails (e.g. zero-sized image).
pub fn encode_rgba_to_sixel(rgba: &[u8], width: u32, height: u32) -> Result<String, SixelError> {
    let image = SixelImage::try_from_rgba(rgba.to_vec(), width as usize, height as usize)?;
    image.encode()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn red_pixel_encodes_without_error() {
        let rgba = vec![255u8, 0, 0, 255];
        let result = encode_rgba_to_sixel(&rgba, 1, 1);
        assert!(result.is_ok());
        let s = result.unwrap();
        assert!(s.contains('\x1b'), "sixel string should contain ESC");
    }

    #[test]
    fn white_image_encodes() {
        let width = 16u32;
        let height = 16u32;
        let rgba = vec![255u8; (width * height * 4) as usize];
        let result = encode_rgba_to_sixel(&rgba, width, height);
        assert!(result.is_ok());
    }
}
