use cairo::{Content, Context, RecordingSurface, Rectangle};
use gdal::Dataset;
use rsvg::Loader;
use std::collections::HashMap;

pub struct Cache {
    pub hillshading_dataset: Option<Dataset>,
    pub svg_map: HashMap<String, RecordingSurface>,
}

impl Cache {
    pub fn get_svg(&mut self, path: &str) -> &RecordingSurface {
        let svg_map = &mut self.svg_map;

        let maybe_cached = svg_map.get(path);

        if maybe_cached.is_none() {
            let handle = Loader::new().read_path(path).unwrap();

            let renderer = rsvg::CairoRenderer::new(&handle);

            let dim = renderer.intrinsic_size_in_pixels().unwrap();

            let rect = Rectangle::new(0.0, 0.0, dim.0, dim.1);

            let tile = RecordingSurface::create(Content::ColorAlpha, Some(rect)).unwrap();

            let context = Context::new(&tile).unwrap();

            renderer.render_document(&context, &rect).unwrap();

            svg_map.insert(String::from(path), tile);
        };

        svg_map.get(path).unwrap()
    }
}
