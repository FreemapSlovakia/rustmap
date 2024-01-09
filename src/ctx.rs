use cairo::Context;

pub struct Ctx {
    pub context: Context,
    pub bbox: (f64, f64, f64, f64),
    pub size: (u32, u32),
}
