//! Demo app for showcasing [`icon_render`] crate.

use std::io::Read as _;

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    let text = text.trim();

    let output_path: std::path::PathBuf = envfury::must("TRAY_ICON_OUTPUT")?;

    let width: u32 = envfury::or("TRAY_ICON_WIDTH", 32)?;
    let height: u32 = envfury::or("TRAY_ICON_HEIGHT", 32)?;

    let mut font_system = cosmic_text::FontSystem::new();

    icon_render::load_font(font_system.db_mut());

    let mut cache = cosmic_text::SwashCache::new();

    let rgba_data = icon_render::render_text(text, &mut font_system, &mut cache, width, height);

    let img = image::RgbaImage::from_raw(width, height, rgba_data.into_vec())
        .ok_or_else(|| color_eyre::eyre::eyre!("Failed to create image from raw data"))?;

    img.save(&output_path)?;

    println!("Image saved to: {}", output_path.display());
    println!("Dimensions: {}x{}", width, height);

    Ok(())
}
