#[derive(Debug, PartialEq)]
pub struct HitWindow {
    pub x300: f64,
    pub x100: f64,
    pub x50: f64,
}

fn diff_map(difficulty: f64, min: f64, mid: f64, max: f64) -> f64 {
    if difficulty > 5.0 {
        mid + (max - mid) * (difficulty - 5.0) / 5.0
    } else if difficulty < 5.0 {
        mid - (mid - min) * (5.0 - difficulty) / 5.0
    } else {
        mid
    }
}

impl HitWindow {
    pub fn from_od(od: f32) -> Self {
        let x300 = diff_map(f64::from(od), 80.0, 50.0, 20.0);
        let x100 = diff_map(f64::from(od), 140.0, 100.0, 60.0);
        let x50 = diff_map(f64::from(od), 200.0, 150.0, 100.0);

        let _xmiss = diff_map(od as f64, 400.0, 400.0, 400.0);

        HitWindow {
            x300,
            x100,
            x50,
        }
    }
}

impl Default for HitWindow {
    fn default() -> Self {
        Self {
            x300: 0.0,
            x100: 0.0,
            x50: 0.0,
        }
    }
}

#[cfg(test)]
mod test {

    use test_case::case;

    use super::HitWindow;
    
    // NM
    #[case(0.0, HitWindow { x300: 80.0, x100: 140.0, x50: 200.0 })]
    #[case(1.0, HitWindow { x300: 74.0, x100: 132.0, x50: 190.0 })]
    #[case(2.0, HitWindow { x300: 68.0, x100: 124.0, x50: 180.0 })]
    #[case(3.0, HitWindow { x300: 62.0, x100: 116.0, x50: 170.0 })]
    #[case(9.0, HitWindow { x300: 26.0, x100: 68.0, x50: 110.0 })]
    #[case(10.0, HitWindow { x300: 20.0, x100: 60.0, x50: 100.0 })]
    fn test_hitwindow_calculation_from_od(od: f32, expected: HitWindow) {
        assert_eq!(HitWindow::from_od(od), expected);
    }
}
