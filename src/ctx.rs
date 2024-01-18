use crate::cache::Cache;
use cairo::Context;
use std::cell::RefCell;

pub struct Ctx<'a> {
    pub context: Context,
    pub bbox: (f64, f64, f64, f64),
    pub size: (u32, u32),
    pub zoom: u32,
    pub scale: f64,
    pub cache: &'a RefCell<Cache>,
}
