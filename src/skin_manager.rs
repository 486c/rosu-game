use std::path::Path;

use crate::{graphics::Graphics, texture::Texture};

/// Handles loading a skin & skin settings from an osu skin
/// If texture requested image is not found will fallback to the 
/// default skin
pub struct SkinManager {
    pub hit_circle: Texture,
}

impl SkinManager {
    pub fn from_path(path: impl AsRef<Path>, graphics: &Graphics) -> Self {
        let hit_circle = { 
            let path = path.as_ref().join("hitcircle.png");
            
            let bytes = if path.exists() {
                std::fs::read(path).unwrap()
            } else {
                include_bytes!("../skin/hitcircle.png").to_vec()
            };

            Texture::from_bytes(&bytes, graphics)
        };

        Self {
            hit_circle,
        }
    }
}
