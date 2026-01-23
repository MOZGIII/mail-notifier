//! Icon utilities for the tray application.

/// The width if the icon image.
pub const WIDTH: u32 = 32;

/// The height if the icon image.
pub const HEIGHT: u32 = 32;

/// Produce a default idle icon.
pub fn idle() -> icon_render_loop::Data {
    let rgba = vec![255u8; WIDTH as usize * HEIGHT as usize * 4].into_boxed_slice();
    icon_render_loop::Data {
        pixels: rgba,
        width: WIDTH,
        height: HEIGHT,
    }
}

/// Convert icon data to a tray icon.
pub fn from_render_loop_data(
    data: icon_render_loop::Data,
) -> Result<tray_icon::Icon, tray_icon::BadIcon> {
    let icon_render_loop::Data {
        pixels,
        width,
        height,
    } = data;

    tray_icon::Icon::from_rgba(pixels.into_vec(), width, height)
}
