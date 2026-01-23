//! Icon image rendering logic.

use crate::icon_data::IconData;

/// The width if the icon image.
const WIDTH: u32 = 32;

/// The height if the icon image.
const HEIGHT: u32 = 32;

/// Produce a default idle icon.
pub fn idle_icon() -> IconData {
    let rgba = vec![255u8; WIDTH as usize * HEIGHT as usize * 4].into_boxed_slice();
    IconData {
        pixels: rgba,
        width: WIDTH,
        height: HEIGHT,
    }
}

/// Run a blocking render loop that
pub fn render_loop(
    mut input_text_receiver: impl FnMut() -> Option<String>,
    mut image_data_sender: impl FnMut(IconData) -> std::ops::ControlFlow<()>,
) {
    let mut font_system = cosmic_text::FontSystem::new();
    let mut cache = cosmic_text::SwashCache::new();

    icon_render::load_font(font_system.db_mut());

    loop {
        let Some(input_text) = (input_text_receiver)() else {
            break;
        };

        let pixels =
            icon_render::render_text(&input_text, &mut font_system, &mut cache, WIDTH, HEIGHT);

        let data = IconData {
            pixels,
            width: WIDTH,
            height: HEIGHT,
        };

        if (image_data_sender)(data).is_break() {
            break;
        }
    }
}
