use cgmath::Vector3;

#[derive(Copy, Clone, Debug)]
pub struct Rgb {
    inner: Vector3<u8>,
}

macro_rules! parse_color {
    ($line:expr) => {{
        let trimmed = $line.trim();
        let comment = trimmed.find("//");

        if let Some(c) = comment {
            &trimmed[0..c].trim()
        } else {
            trimmed
        }
    }}
}

impl Rgb {
    pub fn r(&self) -> u8 {
        self.inner.x
    }

    pub fn g(&self) -> u8 {
        self.inner.y
    }

    pub fn b(&self) -> u8 {
        self.inner.z
    }

    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self {
            inner: Vector3::new(r, g ,b)
        }
    }

    pub fn parse(line: &str) -> Option<Self> {
        let mut split = line.split(',');

        let r = parse_color!(split.next()?);
        let g = parse_color!(split.next()?);
        let b = parse_color!(split.next()?);

        Some(Self {
            inner: Vector3::new(
                r.parse().unwrap(),
                g.parse().unwrap(),
                b.parse().unwrap()
            )
        })
    }

    pub fn to_gpu_values(&self) -> [f32; 3] {
        [
            (self.r() as f32 / 255.0) as f32,
            (self.g() as f32 / 255.0) as f32,
            (self.b() as f32 / 255.0) as f32,
        ]
    }

    pub fn to_egui_color(&self) -> egui::Color32 {
        egui::Color32::from_rgb(self.r(), self.g(), self.b())
    }
}

pub fn mix_colors_linear(color1: &Rgb, color2: &Rgb, ratio: f32) -> Rgb {
    let inv_ratio = 1.0 - ratio;

    Rgb::new(
        ((color1.r() as f32 * ratio) + (color2.r() as f32 * inv_ratio)) as u8,
        ((color1.g() as f32 * ratio) + (color2.g() as f32 * inv_ratio)) as u8,
        ((color1.b() as f32 * ratio) + (color2.b() as f32 * inv_ratio)) as u8,
    )
}

impl Default for Rgb {
    fn default() -> Self {
        Self::new(255, 255, 255)
    }
}

impl From<[u8; 3]> for Rgb {
    fn from(value: [u8; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

impl From<&[u8; 3]> for Rgb {
    fn from(value: &[u8; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

#[test]
fn test_color_parse() {
    let s = "254, 255, 255";
    let parsed = Rgb::parse(s).unwrap();
    assert_eq!(parsed.r(), 254);
    assert_eq!(parsed.g(), 255);
    assert_eq!(parsed.b(), 255);

    let s = "254,  255,  10   ";
    let parsed = Rgb::parse(s).unwrap();
    assert_eq!(parsed.r(), 254);
    assert_eq!(parsed.g(), 255);
    assert_eq!(parsed.b(), 10);

    let s = "254,  255,      10   // comment";
    let parsed = Rgb::parse(s).unwrap();
    assert_eq!(parsed.r(), 254);
    assert_eq!(parsed.g(), 255);
    assert_eq!(parsed.b(), 10);
}
