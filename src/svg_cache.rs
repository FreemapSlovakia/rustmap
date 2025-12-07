use cairo::{Content, Context, RecordingSurface, Rectangle};
use rsvg::Loader;
use std::{cell::RefCell, collections::HashMap};

thread_local! {
    pub static SVG_CACHE_THREAD_LOCAL: RefCell<SvgCache> = {
        RefCell::new(SvgCache::new())
    };
}

pub struct SvgCache {
    svg_map: HashMap<String, RecordingSurface>,
}

impl SvgCache {
    pub fn new() -> SvgCache {
        Self {
            svg_map: HashMap::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> &RecordingSurface {
        let svg_map = &mut self.svg_map;

        let maybe_cached = svg_map.get(key);

        if maybe_cached.is_none() {
            let (path, stylesheet) = key.split_once('|').unwrap_or((key, ""));

            let mut handle = Loader::new().read_path(path).unwrap();

            if !stylesheet.is_empty() {
                handle.set_stylesheet(stylesheet).unwrap();
            }

            let renderer = rsvg::CairoRenderer::new(&handle);

            let dim = renderer.intrinsic_size_in_pixels().unwrap();

            let rect = Rectangle::new(0.0, 0.0, dim.0, dim.1);

            let surface = RecordingSurface::create(Content::ColorAlpha, Some(rect)).unwrap();

            let context = Context::new(&surface).unwrap();

            renderer.render_document(&context, &rect).unwrap();

            svg_map.insert(String::from(key), surface);
        };

        svg_map.get(key).unwrap()
    }
}
