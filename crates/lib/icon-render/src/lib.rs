//! Library for rendering (tray) icon images with text.

/// Embedded font data for the default font.
static FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

/// Loads the default font for rendering into a `fontdb`
/// of some [`cosmic_text::FontSystem`].
pub fn load_font(db: &mut cosmic_text::fontdb::Database) {
    db.load_font_data(FONT_DATA.to_vec());
}

/// Renders text on a white background as an RGBA image.
pub fn render_text(
    text: &str,
    font_system: &mut cosmic_text::FontSystem,
    cache: &mut cosmic_text::SwashCache,
    width: u32,
    height: u32,
) -> Box<[u8]> {
    let len = text.len();
    let scale = if len <= 2 {
        0.6
    } else if len <= 3 {
        0.45
    } else if len <= 4 {
        0.35
    } else if len <= 5 {
        0.3
    } else {
        0.25
    };

    let mut buffer = cosmic_text::Buffer::new_empty(cosmic_text::Metrics::new(
        height as f32 * scale,
        height as f32 * 1.0,
    ));
    let mut buffer = buffer.borrow_with(font_system);

    buffer.set_size(Some(width as f32), Some(height as f32));
    buffer.set_wrap(cosmic_text::Wrap::None);

    let attrs = cosmic_text::Attrs::new();
    buffer.set_text(
        text,
        &attrs,
        cosmic_text::Shaping::Advanced,
        Some(cosmic_text::Align::Center),
    );
    buffer.shape_until_scroll(false);

    let mut pixels = vec![255u8; (width * height * 4) as usize];

    buffer.draw(
        cache,
        cosmic_text::Color::rgb(0, 0, 0),
        |x, y, w, h, color| {
            let x = x as usize;
            let y = y as usize;
            let w = w as usize;
            let h = h as usize;

            let [r, g, b, a] = color.as_rgba();

            let neg_a = 255u32 - a as u32;

            let apply_a_prepass = |background| ((background as u32 * neg_a) / 255) as u8;

            for gy in 0..h {
                for gx in 0..w {
                    let px = x + gx;
                    let py = y + gy;
                    if px < width as usize && py < height as usize {
                        let idx = (py * width as usize + px) * 4;

                        pixels[idx] = apply_a_prepass(pixels[idx]);
                        pixels[idx + 1] = apply_a_prepass(pixels[idx + 1]);
                        pixels[idx + 2] = apply_a_prepass(pixels[idx + 2]);
                        pixels[idx + 3] = apply_a_prepass(pixels[idx + 3]);

                        pixels[idx] += r; // R
                        pixels[idx + 1] += g; // G
                        pixels[idx + 2] += b; // B
                        pixels[idx + 3] += a; // A
                    }
                }
            }
        },
    );

    pixels.into_boxed_slice()
}
