//! The icon render loop data.

/// An intermediate representation of the icon image data.
pub struct Data {
    /// Image pixels, as the RGBA byte sequences.
    pub pixels: Box<[u8]>,

    /// Image width.
    pub width: u32,

    /// Image height.
    pub height: u32,
}

impl core::fmt::Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data")
            .field("pixels", &format_args!("{} bytes", self.pixels.len()))
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}
