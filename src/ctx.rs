use crate::{bbox::BBox, size::Size, svg_cache::SvgCache};
use cairo::Context;
use gdal::Dataset;
use std::{cell::RefCell, collections::HashMap};

pub struct Ctx<'a> {
    pub context: &'a Context,
    pub bbox: BBox<f64>,
    pub size: Size<u32>,
    pub zoom: u32,
    pub scale: f64,
    pub svg_cache: &'a RefCell<SvgCache>,
    pub shading_data: &'a RefCell<HashMap<String, Dataset>>,
}

impl Ctx<'_> {
    pub fn meters_per_pixel(&self) -> f64 {
        (self.bbox.max_x - self.bbox.min_x) / self.size.width as f64
    }
}
