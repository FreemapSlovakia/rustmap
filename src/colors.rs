use cairo::Context;

pub type Color = (f64, f64, f64);

pub fn hsl_to_rgb(h: u16, s: u8, l: u8) -> Color {
    let h = h as f64 / 360.0; // Convert to range [0, 1]
    let s = s as f64 / 100.0; // Convert to range [0, 1]
    let l = l as f64 / 100.0; // Convert to range [0, 1]

    let (r, g, b) = if s == 0.0 {
        (l, l, l) // Achromatic
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        (
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    };

    // Convert to range [0, 255]
    (r, g, b)
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

lazy_static! {
    pub static ref ADMIN_BORDER: Color = hsl_to_rgb(278, 100, 50);
    pub static ref AEROWAY: Color = hsl_to_rgb(260, 10, 50);
    pub static ref ALLOTMENTS: Color = hsl_to_rgb(50, 45, 88);
    pub static ref AREA_LABEL: Color = hsl_to_rgb(0, 0, 33);
    pub static ref BEACH: Color = hsl_to_rgb(60, 90, 85);
    pub static ref BROWNFIELD: Color = hsl_to_rgb(30, 30, 68);
    pub static ref BUILDING: Color = hsl_to_rgb(0, 0, 50);
    pub static ref BRIDLEWAY: Color = hsl_to_rgb(120, 50, 30);
    pub static ref BRIDLEWAY2: Color = hsl_to_rgb(120, 50, 80);
    pub static ref CLEARCUT: Color = hsl_to_rgb(95, 40, 85);
    pub static ref COLLEGE: Color = hsl_to_rgb(60, 85, 92);
    pub static ref COMMERCIAL: Color = hsl_to_rgb(320, 40, 90);
    pub static ref CONTOUR: Color = hsl_to_rgb(0, 0, 0);
    pub static ref CYCLEWAY: Color = hsl_to_rgb(282, 100, 50);
    pub static ref DAM: Color = hsl_to_rgb(0, 0, 70);
    pub static ref FARMLAND: Color = hsl_to_rgb(60, 70, 95);
    pub static ref FARMYARD: Color = hsl_to_rgb(50, 44, 85);
    pub static ref FOREST: Color = hsl_to_rgb(110, 60, 83);
    pub static ref GLOW: Color = hsl_to_rgb(0, 33, 70);
    pub static ref GRASSY: Color = hsl_to_rgb(100, 100, 93);
    pub static ref HEATH: Color = hsl_to_rgb(85, 60, 85);
    pub static ref HOSPITAL: Color = hsl_to_rgb(50, 85, 92);
    pub static ref INDUSTRIAL: Color = hsl_to_rgb(0, 0, 85);
    pub static ref LANDFILL: Color = hsl_to_rgb(0, 30, 75);
    pub static ref MILITARY: Color = hsl_to_rgb(0, 96, 39);
    pub static ref NONE: Color = hsl_to_rgb(0, 100, 100);
    pub static ref ORCHARD: Color = hsl_to_rgb(90, 75, 85);
    pub static ref PARKING_STROKE: Color = hsl_to_rgb(0, 30, 75);
    pub static ref PARKING: Color = hsl_to_rgb(0, 20, 88);
    pub static ref PIER: Color = hsl_to_rgb(0, 0, 0);
    pub static ref PIPELINE: Color = hsl_to_rgb(0, 0, 50);
    pub static ref PISTE: Color = hsl_to_rgb(0, 255, 255);
    pub static ref PISTE2: Color = hsl_to_rgb(0, 0, 62);
    pub static ref PITCH_STROKE: Color = hsl_to_rgb(110, 35, 50);
    pub static ref PITCH: Color = hsl_to_rgb(110, 35, 75);
    pub static ref POWER_LINE: Color = hsl_to_rgb(0, 0, 0);
    pub static ref POWER_LINE_MINOR: Color = hsl_to_rgb(0, 0, 50);
    pub static ref PROTECTED: Color = hsl_to_rgb(120, 75, 25);
    pub static ref QUARRY: Color = hsl_to_rgb(0, 0, 78);
    pub static ref RESIDENTIAL: Color = hsl_to_rgb(100, 0, 91);
    pub static ref ROAD: Color = hsl_to_rgb(40, 60, 50);
    pub static ref RUIN: Color = hsl_to_rgb(0, 0, 60);
    pub static ref SCREE: Color = hsl_to_rgb(0, 0, 90);
    pub static ref SCRUB: Color = hsl_to_rgb(100, 70, 86);
    pub static ref SILO_STROKE: Color = hsl_to_rgb(50, 20, 30);
    pub static ref SILO: Color = hsl_to_rgb(50, 20, 50);
    pub static ref SUPERROAD: Color = hsl_to_rgb(10, 60, 60);
    pub static ref TRACK: Color = hsl_to_rgb(0, 33, 25);
    pub static ref WATER_LABEL_HALO: Color = hsl_to_rgb(216, 30, 100);
    pub static ref WATER_LABEL: Color = hsl_to_rgb(216, 100, 50);
    pub static ref WATER_SLIDE: Color = hsl_to_rgb(180, 50, 50);
    pub static ref WATER: Color = hsl_to_rgb(216, 65, 70);
    pub static ref RAIL_GLOW: Color = hsl_to_rgb(0, 100, 100);
    pub static ref TRAM: Color = hsl_to_rgb(0, 0, 20);
    pub static ref RAILWAY_DISUSED: Color = hsl_to_rgb(0, 0, 30);
    pub static ref RAIL: Color = hsl_to_rgb(0, 0, 0);
    pub static ref CONSTRUCTION_ROAD_1: Color = hsl_to_rgb(60, 100, 50);
    pub static ref CONSTRUCTION_ROAD_2: Color = hsl_to_rgb(0, 0, 40);
    pub static ref BARRIERWAY: Color = hsl_to_rgb(0, 100, 50);
    pub static ref BLACK: Color = hsl_to_rgb(0, 0, 0);
    pub static ref WHITE: Color = hsl_to_rgb(0, 100, 100);
}

pub trait ContextExt {
    fn set_source_color(&self, color: Color);

    fn set_source_color_a(&self, color: Color, alpha: f64);
}

impl ContextExt for Context {
    fn set_source_color(&self, color: Color) {
        self.set_source_rgb(color.0, color.1, color.2);
    }

    fn set_source_color_a(&self, color: Color, alpha: f64) {
        self.set_source_rgba(color.0, color.1, color.2, alpha);
    }
}
