use cairo::{Content, RecordingSurface, Rectangle};
use rsvg::Loader;
use std::{collections::HashMap, path::PathBuf};

pub struct SvgCache {
    base: PathBuf,
    svg_map: HashMap<String, RecordingSurface>,
}

impl SvgCache {
    pub fn new(base: impl Into<PathBuf>) -> SvgCache {
        Self {
            base: base.into(),
            svg_map: HashMap::new(),
        }
    }

    pub fn set_base(&mut self, base: impl Into<PathBuf>) {
        self.base = base.into();
        self.svg_map.clear();
    }

    pub fn get(&mut self, key: &str) -> &RecordingSurface {
        let svg_map = &mut self.svg_map;

        if !svg_map.contains_key(key) {
            let (path, stylesheet) = key.split_once('|').unwrap_or((key, ""));

            let full_path = self.base.join(path);

            let mut handle = Loader::new().read_path(full_path).unwrap();

            if !stylesheet.is_empty() {
                handle.set_stylesheet(stylesheet).unwrap();
            }

            let renderer = rsvg::CairoRenderer::new(&handle);
            let dim = renderer.intrinsic_size_in_pixels().unwrap();
            let rect = Rectangle::new(0.0, 0.0, dim.0, dim.1);
            let surface = RecordingSurface::create(Content::ColorAlpha, Some(rect)).unwrap();
            let context = cairo::Context::new(&surface).unwrap();

            renderer.render_document(&context, &rect).unwrap();

            svg_map.insert(String::from(key), surface);
        }

        svg_map.get(key).unwrap()
    }
}
