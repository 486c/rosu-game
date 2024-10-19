pub mod graphics;
pub mod osu_renderer;
pub mod hit_objects;
pub mod texture;
pub mod math;
pub mod camera;
pub mod rgb;
pub mod quad_renderer;
pub mod quad_instance;
pub mod skin_manager;
pub mod vertex;
pub mod config;
pub mod hit_circle_instance;
pub mod slider_instance;
pub mod timer;
pub mod skin_ini;
pub mod processor;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        mod egui_state;
        pub mod osu_state;
        mod ui;
        mod screen;
        mod song_select_state;
        mod osu_db;
        mod song_importer_ui;
        mod osu_cursor_renderer;
        mod frameless_source;
        mod osu_input;
    }
}

