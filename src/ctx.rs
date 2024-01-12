use cairo::Context;

use crate::cache::Cache;

pub struct Ctx<'a> {
    pub context: Context,
    pub bbox: (f64, f64, f64, f64),
    pub size: (u32, u32),
    pub zoom: u32,
    pub cache: &'a mut Cache,
}
