use cgmath::Vector2;

pub const OSU_COORDS_WIDTH: f32 = 512.0;
pub const OSU_COORDS_HEIGHT: f32 = 384.0;

pub const OSU_PLAYFIELD_BORDER_TOP_PERCENT: f32 = 0.117;
pub const OSU_PLAYFIELD_BORDER_BOTTOM_PERCENT: f32 = 0.0834;

pub fn lerp(a: f64, b: f64, v: f64) -> f64 {
    a + v * (b - a)
}

pub fn diff_rate(diff: f32, min: f32, mid: f32, max: f32) -> f32 {
	if diff > 5.0 {
		return mid + (max-mid)*(diff-5.0)/5.0
	}

	if diff < 5.0 {
		return mid - (mid-min)*(5.0-diff)/5.0
	}

	return mid
}

pub fn calc_hitcircle_diameter(cs: f32) -> f32 {
    //diff_rate(cs, 54.4, 32.0, 9.6) * 1.00041
    ((1.0 - 0.7 * (cs - 5.0) / 5.0) / 2.0) * 128.0 * 1.00041
}

pub fn calc_playfield_scale_factor(screen_w: f32, screen_h: f32) -> f32 {
    let top_border_size = OSU_PLAYFIELD_BORDER_TOP_PERCENT * screen_h;
    let bottom_border_size = OSU_PLAYFIELD_BORDER_BOTTOM_PERCENT * screen_h;

    let engine_screen_w = screen_w;
    let engine_screen_h = screen_h - bottom_border_size - top_border_size;

    let scale_factor = if screen_w / OSU_COORDS_WIDTH > engine_screen_h / OSU_COORDS_HEIGHT {
        engine_screen_h / OSU_COORDS_HEIGHT
    } else {
        engine_screen_w / OSU_COORDS_WIDTH
    };

    return scale_factor;
}

pub fn calc_playfield(screen_w: f32, screen_h: f32) -> (f32, Vector2<f32>) {
    let scale = calc_playfield_scale_factor(screen_w, screen_h);

    let scaled_height = OSU_COORDS_HEIGHT as f32 * scale;
    let scaled_width = OSU_COORDS_WIDTH as f32 * scale;

    let bottom_border_size = OSU_PLAYFIELD_BORDER_BOTTOM_PERCENT * screen_h as f32;
    let playfield_y_offset = (screen_h / 2.0 - (scaled_height / 2.0)) - bottom_border_size;
    
    let offsets = Vector2::new(
        (screen_w - scaled_width) / 2.0,
        (screen_h - scaled_height) / 2.0 + playfield_y_offset
    );

    (scale, offsets)
}

pub fn calc_direction_degree(p1: Vector2<f32>, p2: Vector2<f32>) -> f32 {
    let angle_rad = (p2.y - p1.y).atan2(p2.x - p1.x);
    let mut angle_deg = angle_rad.to_degrees();

    if angle_deg < 0.0 {
        angle_deg += 360.0;
    }

    angle_deg
}

pub fn calc_opposite_direction_degree(p1: Vector2<f32>, p2: Vector2<f32>) -> f32 {
    (calc_direction_degree(p1, p2) + 180.0) % 360.0
}

/// Return preempt and fadein based on AR
pub fn calculate_preempt_fadein(ar: f32) -> (f32, f32) {
    if ar > 5.0 {
        (
            1200.0 - 750.0 * (ar - 5.0) / 5.0,
            800.0 - 500.0 * (ar - 5.0) / 5.0,
        )
    } else if ar < 5.0 {
        (
            1200.0 + 600.0 * (5.0 - ar) / 5.0,
            800.0 + 400.0 * (5.0 - ar) / 5.0,
        )
    } else {
        (1200.0, 800.0)
    }
}

#[inline]
pub fn calc_progress(current: f64, start: f64, end: f64) -> f64 {
    (current - start) / (end - start)
}

#[inline]
pub fn calc_fade_alpha(
    time: f64, 
    start: f64,
    fade_in_ms: f32, 
    stay_ms: f32, 
    fade_out_ms: f32
) -> f64 {
    let fade_in_end = start + fade_in_ms as f64;
    let fade_out_start = fade_in_end + stay_ms as f64;
    let fade_out_end = fade_out_start + fade_out_ms as f64;

    if time <= fade_in_end {
        calc_progress(time, start, fade_in_end)
    } else if time >= fade_out_start {
        1.0 - calc_progress(time, fade_out_start, fade_out_end)
    } else {
        1.0
    }
}


#[test]
pub fn test_progress() {
    assert_eq!(calc_progress(50.0, 0.0, 100.0), 0.50);
}

#[test]
pub fn test_directiondegrees() {
    let p1 = Vector2::new(0.0, 0.0);
    let p2 = Vector2::new(0.0, 6.0);

    assert_eq!(calc_opposite_direction_degree(p1, p2), 270.0)
}
