use std::{io, path::Path};
use crate::{graphics::Graphics, skin_ini::SkinIni, texture::{AtlasTexture, Texture}};
use image::load_from_memory;

macro_rules! load_or_fallback_image {
    ($path:expr, $name: expr, $graphics:expr) => {{
        load_or_fallback_texture!($path, $name, $name, $graphics)
    }};
    ($path:expr, $name: expr, $fallback_name: expr, $graphics:expr) => {{
        let path = $path.as_ref().join($name);

        let bytes = if path.exists() {
            std::fs::read(path).unwrap()
        } else {
            let fallback_path = format!("./skin/{}", $fallback_name);
            std::fs::read(&fallback_path)
                .expect(&format!("Failed to load fallback image from {}", &fallback_path))
        };

        load_from_memory(&bytes).unwrap()
    }}
}

macro_rules! load_or_fallback_texture {
    ($path:expr, $name: expr, $graphics:expr) => {{
        load_or_fallback_texture!($path, $name, $name, $graphics)
    }};
    ($path:expr, $name: expr, $fallback_name: expr, $graphics:expr) => {{
        let path = $path.as_ref().join($name);

        let bytes = if path.exists() {
            std::fs::read(path).unwrap()
        } else {
            let fallback_path = format!("./skin/{}", $fallback_name);
            std::fs::read(&fallback_path)
                .expect(&format!("Failed to load fallback image from {}", &fallback_path))
        };

        Texture::from_bytes(&bytes, $graphics)
    }}
}

pub struct Judgments {
    pub hit_300: Texture,
    pub hit_100: Texture,
    pub hit_50: Texture,
    pub hit_miss: Texture
}

/// Handles loading a skin & skin settings from an osu skin
/// If texture requested image is not found will fallback to the 
/// default skin
pub struct SkinManager {
    pub ini: SkinIni,
    pub hit_circle: Texture,
    pub hit_circle_overlay: Texture,
    pub sliderb0: Texture,
    pub cursor: Texture,
    pub cursor_trail: Texture,
    pub debug_texture: Texture,
    pub judgments: Judgments,
    pub judgments_atlas: AtlasTexture,

    pub debug_texture2: Texture,
}

impl SkinManager {
    pub fn from_path(path: impl AsRef<Path>, graphics: &Graphics) -> Self {
        let skin_ini = {
            let path = {
                if path.as_ref().join("skin.ini").exists() {
                    path.as_ref().join("skin.ini")
                } else if path.as_ref().join("Skin.ini").exists() {
                    path.as_ref().join("Skin.ini")
                } else {
                    path.as_ref().join("FIXMESOMEDAYPLS.ini")
                }
            };

            let skin_ini_bytes = std::fs::read(
                path
            );

            match skin_ini_bytes {
                Ok(bytes) => {
                    let skin_ini = SkinIni::parse(&bytes)
                        .inspect_err(|e| println!("Failed to deserialize skin.ini: {e}")).unwrap_or(SkinIni::default());

                    skin_ini
                },
                Err(_) => {
                    SkinIni::default()
                },
            }

        };

        // We need to handle two situations:
        // 1. Hit Circle Overlay is present =>
        //     Load it and use it
        // 2. Hit Circle Overlay is not present =>
        //     We SHOULD NOT fallback to the default skin
        //     because it might that skin is intentially not using overlay
        //     In that case we loading empty 1x1 image
        let hit_circle = load_or_fallback_texture!(path, "hitcircle.png", graphics);
        let hit_circle_overlay = load_or_fallback_texture!(path, "hitcircleoverlay.png", "empty.png", graphics);

        let _approach_circle = load_or_fallback_texture!(path, "approachcircle.png", graphics);
        let sliderb0 = load_or_fallback_texture!(path, "sliderb0.png", graphics);
        let cursor = load_or_fallback_texture!(path, "cursor.png", graphics);
        let cursor_trail = load_or_fallback_texture!(path, "cursortrail.png", graphics);

        // Loading judgments

        let judgments = Judgments {
            hit_miss: load_or_fallback_texture!(path, "unexistenttexture.pnng", "hit0.png", graphics),
            hit_300: load_or_fallback_texture!(path, "unexistenttexture.pnng", "hit300.png", graphics),
            hit_100: load_or_fallback_texture!(path, "unexistenttexture.pnng", "hit100.png", graphics),
            hit_50: load_or_fallback_texture!(path, "unexistenttexture.pnng", "hit50.png", graphics),
        };

        let debug_texture = load_or_fallback_texture!(path, "debug.png", graphics);
        let debug_texture2 = load_or_fallback_texture!(path, "debug2.png", graphics);

        let hit_miss = load_or_fallback_image!(path, "unexistenttexture.pnng", "hit0.png", graphics);
        let hit_300 = load_or_fallback_image!(path, "unexistenttexture.pnng", "hit300.png", graphics);
        let hit_100 = load_or_fallback_image!(path, "unexistenttexture.pnng", "hit100.png", graphics);
        let hit_50 = load_or_fallback_image!(path, "unexistenttexture.pnng", "hit50.png", graphics);

        let judgments_atlas = AtlasTexture::from_images(
            graphics, 
            &[hit_300, hit_100, hit_50, hit_miss]
        );

        Self {
            ini: skin_ini,
            hit_circle,
            hit_circle_overlay,
            sliderb0,
            cursor,
            cursor_trail,
            debug_texture,
            debug_texture2,
            judgments,
            judgments_atlas        
        }
    }
}
