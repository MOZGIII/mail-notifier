//! The icon data.

/// An intermediate representation of the icon image data.
pub struct IconData {
    /// Image pixels, as the RGBA byte sequences.
    pub pixels: Box<[u8]>,

    /// Image width.
    pub width: u32,

    /// Image height.
    pub height: u32,
}

impl core::fmt::Debug for IconData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconData")
            .field("pixels", &format_args!("{} bytes", self.pixels.len()))
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl TryFrom<IconData> for tray_icon::Icon {
    type Error = tray_icon::BadIcon;

    fn try_from(value: IconData) -> Result<Self, Self::Error> {
        let IconData {
            pixels,
            width,
            height,
        } = value;

        tray_icon::Icon::from_rgba(pixels.into_vec(), width, height)
    }
}
