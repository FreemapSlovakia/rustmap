use cairo::{Content, RecordingSurface, Rectangle};
use rsvg::{Loader, LoadingError, RenderingError};
use std::{collections::HashMap, path::PathBuf};

pub struct SvgCache {
    base: PathBuf,
    svg_map: HashMap<String, RecordingSurface>,
}

#[derive(Debug, thiserror::Error)]
pub enum SvgCacheError {
    #[error("Error loading SVG ({layer}): {source}")]
    LoadingError {
        layer: String,
        #[source]
        source: LoadingError,
    },

    #[error("Cairo error ({layer}): {source}")]
    CairoError {
        layer: String,
        #[source]
        source: cairo::Error,
    },

    #[error("Error rendering SVG ({layer}): {source}")]
    RenderingError {
        layer: String,
        #[source]
        source: RenderingError,
    },
}

impl SvgCache {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            svg_map: HashMap::new(),
        }
    }

    pub fn set_base(&mut self, base: impl Into<PathBuf>) {
        self.base = base.into();
        self.svg_map.clear();
    }

    pub fn get(&mut self, key: &str) -> Result<&RecordingSurface, SvgCacheError> {
        let svg_map = &mut self.svg_map;
        let layer = key.to_string();

        if !svg_map.contains_key(key) {
            let (path, stylesheet) = key.split_once('|').unwrap_or((key, ""));

            let map_loading_error = |err| SvgCacheError::LoadingError {
                layer: layer.clone(),
                source: err,
            };

            let map_cairo_error = |err| SvgCacheError::CairoError {
                layer: layer.clone(),
                source: err,
            };

            let full_path = self.base.join(path);

            let mut handle = Loader::new()
                .read_path(full_path)
                .map_err(map_loading_error)?;

            if !stylesheet.is_empty() {
                handle
                    .set_stylesheet(stylesheet)
                    .map_err(map_loading_error)?;
            }

            let renderer = rsvg::CairoRenderer::new(&handle);
            let dim = renderer.intrinsic_size_in_pixels().unwrap_or((16.0, 16.0));
            let rect = Rectangle::new(0.0, 0.0, dim.0, dim.1);
            let surface = RecordingSurface::create(Content::ColorAlpha, Some(rect))
                .map_err(map_cairo_error)?;
            let context = cairo::Context::new(&surface).map_err(map_cairo_error)?;

            renderer.render_document(&context, &rect).map_err(|err| {
                SvgCacheError::RenderingError {
                    layer: layer.clone(),
                    source: err,
                }
            })?;

            svg_map.insert(String::from(key), surface);
        }

        Ok(svg_map.get(key).expect("svg from map"))
    }
}
