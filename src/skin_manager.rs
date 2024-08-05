use std::{io, path::Path};
use crate::{graphics::Graphics, skin_ini::SkinIni, texture::Texture};

macro_rules! load_or_fallback {
    ($path:expr, $name: expr, $graphics:expr) => {{
        load_or_fallback!($path, $name, $name, $graphics)
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

/// Handles loading a skin & skin settings from an osu skin
/// If texture requested image is not found will fallback to the 
/// default skin
pub struct SkinManager {
    pub ini: SkinIni,
    pub hit_circle: Texture,
    pub hit_circle_overlay: Texture,
    pub sliderb0: Texture,
}

impl SkinManager {
    pub fn from_path(path: impl AsRef<Path>, graphics: &Graphics) -> Self {

        let skin_ini = {
            let skin_ini_bytes = std::fs::read(
                path.as_ref().join("skin.ini")
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
        let hit_circle = load_or_fallback!(path, "hitcircle.png", graphics);
        let hit_circle_overlay = load_or_fallback!(path, "hitcircleoverlay.png", "empty.png", graphics);

        let _approach_circle = load_or_fallback!(path, "approachcircle.png", graphics);
        let sliderb0 = load_or_fallback!(path, "sliderb0.png", graphics);

        Self {
            ini: skin_ini,
            hit_circle,
            hit_circle_overlay,
            sliderb0,
        }
    }
}
