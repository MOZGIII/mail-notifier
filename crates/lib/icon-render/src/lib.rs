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

/// Parameters for [`render_text`].
pub struct RenderTextParams<'a> {
    /// The text to render on the icon.
    pub text: &'a str,

    /// The width of the output image in pixels.
    pub width: u32,

    /// The height of the output image in pixels.
    pub height: u32,

    /// When `true` the background is red and the text is white, signalling
    /// elevated attention. Otherwise the background is white with black text.
    pub attention: bool,
}

/// Renders text on an icon image as RGBA pixels.
///
/// The resulting image has rounded corners (transparent outside the
/// rounded rectangle).
pub fn render_text(
    params: &RenderTextParams<'_>,
    font_system: &mut cosmic_text::FontSystem,
    cache: &mut cosmic_text::SwashCache,
) -> Box<[u8]> {
    let RenderTextParams {
        text,
        width,
        height,
        attention,
    } = *params;
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

        // cosmic_text provides non-premultiplied RGBA: the RGB channels
        // carry the raw text colour and the alpha channel carries the glyph
        // coverage.  We composite with standard alpha-over:
        //   dst = dst * (1 − src_a) + src * src_a
        let a32 = a as u32;
        let neg_a = 255u32 - a32;

        let blend = |dst: u8, src: u8| ((dst as u32 * neg_a + src as u32 * a32) / 255) as u8;

        for gy in 0..h {
            for gx in 0..w {
                let px = x + gx;
                let py = y + gy;
                if px < width as usize && py < height as usize {
                    let idx = (py * width as usize + px) * 4;

                    pixels[idx] = blend(pixels[idx], r);
                    pixels[idx + 1] = blend(pixels[idx + 1], g);
                    pixels[idx + 2] = blend(pixels[idx + 2], b);
                    // Keep alpha fully opaque: `a` controls how much
                    // the RGB channels shift toward the text colour,
                    // but the icon surface must stay opaque so the
                    // desktop background doesn't bleed through
                    // antialiased text edges.
                    pixels[idx + 3] = blend(pixels[idx + 3], 255);
                }
            }
        }
    });

    apply_rounded_corners(&mut pixels, width, height);

    pixels.into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (cosmic_text::FontSystem, cosmic_text::SwashCache) {
        let mut font_system = cosmic_text::FontSystem::new();
        let cache = cosmic_text::SwashCache::new();
        load_font(font_system.db_mut());
        (font_system, cache)
    }

    /// Helper: get the RGBA tuple at pixel (x, y).
    fn pixel_at(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let idx = (y * width + x) as usize * 4;
        [
            pixels[idx],
            pixels[idx + 1],
            pixels[idx + 2],
            pixels[idx + 3],
        ]
    }

    #[test]
    fn corner_pixels_are_transparent() {
        let (mut fs, mut cache) = setup();
        let pixels = render_text(
            &RenderTextParams {
                text: "1",
                width: 32,
                height: 32,
                attention: false,
            },
            &mut fs,
            &mut cache,
        );
        // (0,0) is in the rounded-off corner.
        let [_, _, _, a] = pixel_at(&pixels, 32, 0, 0);
        assert_eq!(a, 0, "top-left corner pixel should be transparent");
    }

    #[test]
    fn background_pixels_are_opaque_white_without_attention() {
        let (mut fs, mut cache) = setup();
        let pixels = render_text(
            &RenderTextParams {
                text: "1",
                width: 32,
                height: 32,
                attention: false,
            },
            &mut fs,
            &mut cache,
        );
        // A pixel on the top edge, centred horizontally, should be inside
        // the rounded rect but far from any glyph.
        let [r, g, b, a] = pixel_at(&pixels, 32, 16, 1);
        assert_eq!(a, 255, "non-corner pixel should be opaque");
        assert_eq!([r, g, b], [255, 255, 255], "background should be white");
    }

    #[test]
    fn background_pixels_are_opaque_red_with_attention() {
        let (mut fs, mut cache) = setup();
        let pixels = render_text(
            &RenderTextParams {
                text: "1",
                width: 32,
                height: 32,
                attention: true,
            },
            &mut fs,
            &mut cache,
        );
        let [r, g, b, a] = pixel_at(&pixels, 32, 16, 1);
        assert_eq!(a, 255, "non-corner pixel should be opaque");
        assert_eq!([r, g, b], [220, 38, 38], "background should be red");
    }

    #[test]
    fn text_pixels_are_darker_than_white_background() {
        let (mut fs, mut cache) = setup();
        let pixels = render_text(
            &RenderTextParams {
                text: "1",
                width: 32,
                height: 32,
                attention: false,
            },
            &mut fs,
            &mut cache,
        );
        // At least one pixel in the centre region should have been darkened
        // by the black text.
        let has_text = (12..20).any(|y| {
            (10..22).any(|x| {
                let [r, _, _, a] = pixel_at(&pixels, 32, x, y);
                a == 255 && r < 200
            })
        });
        assert!(has_text, "some centre pixels should be darkened by text");
    }

    #[test]
    fn attention_text_pixels_are_lighter_than_red_background() {
        let (mut fs, mut cache) = setup();
        let pixels = render_text(
            &RenderTextParams {
                text: "1",
                width: 32,
                height: 32,
                attention: true,
            },
            &mut fs,
            &mut cache,
        );
        // With white-on-red, any text pixel should have channels
        // moving *toward* white (i.e. R ≥ 220, G ≥ 38, B ≥ 38).
        // If compositing is broken (overflow wrapping) some channels will
        // be *less* than the background.
        let has_bad_pixel = (8..24).any(|y| {
            (8..24).any(|x| {
                let [r, g, b, a] = pixel_at(&pixels, 32, x, y);
                // Only check opaque, non-background pixels.
                if a != 255 || (r == 220 && g == 38 && b == 38) {
                    return false;
                }
                r < 220 || g < 38 || b < 38
            })
        });
        assert!(
            !has_bad_pixel,
            "white text on red should never produce channels below the background value \
             (would indicate overflow in alpha compositing)"
        );
    }
}
