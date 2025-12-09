use crate::{bbox::BBox, projectable::TileProjector, size::Size, svg_cache::SvgCache};
use cairo::Context;
use gdal::Dataset;
use postgres::types::ToSql;
use std::{cell::RefCell, collections::HashMap};

pub struct SqlParams {
    params: Vec<Box<dyn ToSql + Sync>>,
}

impl SqlParams {
    pub fn push<T>(&mut self, value: T)
    where
        T: ToSql + Sync + 'static,
    {
        self.params.push(Box::new(value));
    }

    pub fn as_params(&self) -> Vec<&(dyn ToSql + Sync)> {
        self.params
            .iter()
            .map(|param| param.as_ref() as &(dyn ToSql + Sync))
            .collect()
    }
}

pub struct Ctx<'a> {
    pub context: &'a Context,
    pub bbox: BBox<f64>,
    pub size: Size<u32>,
    pub zoom: u32,
    pub scale: f64,
    pub svg_cache: &'a RefCell<SvgCache>,
    pub shading_data: &'a RefCell<HashMap<String, Dataset>>,
    pub tile_projector: TileProjector,
}

impl Ctx<'_> {
    pub fn meters_per_pixel(&self) -> f64 {
        (self.bbox.max_x - self.bbox.min_x) / self.size.width as f64
    }

    pub fn bbox_query_params(&self, buffer_from_param: Option<f64>) -> SqlParams {
        let mut params: Vec<Box<dyn ToSql + Sync>> = vec![
            Box::new(self.bbox.min_x),
            Box::new(self.bbox.min_y),
            Box::new(self.bbox.max_x),
            Box::new(self.bbox.max_y),
        ];

        if let Some(buffer_from_param) = buffer_from_param {
            params.push(Box::new(self.meters_per_pixel() * buffer_from_param));
        }

        SqlParams { params }
    }
}
