use crate::{bbox::BBox, cache::Cache, size::Size};
use cairo::Context;
use std::cell::RefCell;

pub struct Ctx<'a> {
    pub context: Context,
    pub bbox: BBox<f64>,
    pub size: Size<u32>,
    pub zoom: u32,
    pub scale: f64,
    pub cache: &'a RefCell<Cache>,
}

impl Ctx<'_> {
    pub fn meters_per_pixel(&self) -> f64 {
        (self.bbox.max_x - self.bbox.min_x) / self.size.width as f64
    }
}
