//! Library for rendering (tray) icon images with text.

/// Embedded font data for the default font.
static FONT_DATA: &[u8] = include_bytes!("../assets/font.ttf");

/// Loads the default font for rendering into a `fontdb`
/// of some [`cosmic_text::FontSystem`].
pub fn load_font(db: &mut cosmic_text::fontdb::Database) {
    db.load_font_data(FONT_DATA.to_vec());
}

/// Returns `true` if the pixel at `(px, py)` lies inside a rounded rectangle
/// of the given `width`×`height` with corner `radius`.
fn inside_rounded_rect(px: u32, py: u32, width: u32, height: u32, radius: u32) -> bool {
    let r = radius as f32;
    let w = width as f32;
    let h = height as f32;
    let x = px as f32 + 0.5;
    let y = py as f32 + 0.5;

    // Check if the pixel is in one of the four corner regions.
    let (cx, cy) = if x < r && y < r {
        (r, r) // top-left
    } else if x >= w - r && y < r {
        (w - r, r) // top-right
    } else if x < r && y >= h - r {
        (r, h - r) // bottom-left
    } else if x >= w - r && y >= h - r {
        (w - r, h - r) // bottom-right
    } else {
        return true; // not in a corner region
    };

    let dx = x - cx;
    let dy = y - cy;
    dx * dx + dy * dy <= r * r
}

/// Applies a rounded-corner mask in-place on an RGBA pixel buffer:
/// pixels outside the rounded rectangle become fully transparent.
fn apply_rounded_corners(pixels: &mut [u8], width: u32, height: u32) {
    let radius = width.min(height) / 5;
    for py in 0..height {
        for px in 0..width {
            if !inside_rounded_rect(px, py, width, height, radius) {
                let idx = (py * width + px) as usize * 4;
                pixels[idx] = 0;
                pixels[idx + 1] = 0;
                pixels[idx + 2] = 0;
                pixels[idx + 3] = 0;
            }
        }
    }
}

/// Renders text on an icon image as RGBA pixels.
///
/// When `attention` is `true` the background is red and the text is white,
/// signalling elevated attention. Otherwise the background is white with
/// black text.
///
/// The resulting image has rounded corners (transparent outside the
/// rounded rectangle).
pub fn render_text(
    text: &str,
    font_system: &mut cosmic_text::FontSystem,
    cache: &mut cosmic_text::SwashCache,
    width: u32,
    height: u32,
    attention: bool,
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

    // Background colour: red for attention, white otherwise.
    let (bg_r, bg_g, bg_b): (u8, u8, u8) = if attention {
        (220, 38, 38)
    } else {
        (255, 255, 255)
    };

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for chunk in pixels.chunks_exact_mut(4) {
        chunk[0] = bg_r;
        chunk[1] = bg_g;
        chunk[2] = bg_b;
        chunk[3] = 255;
    }

    // Text colour: white on red background, black on white background.
    let text_color = if attention {
        cosmic_text::Color::rgb(255, 255, 255)
    } else {
        cosmic_text::Color::rgb(0, 0, 0)
    };

    buffer.draw(cache, text_color, |x, y, w, h, color| {
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
    });

    apply_rounded_corners(&mut pixels, width, height);

    pixels.into_boxed_slice()
}
