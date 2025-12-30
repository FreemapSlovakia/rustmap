use cairo::{Content, RecordingSurface, Rectangle};
use gio::glib;
use rsvg::{LoadingError, RenderingError};
use std::{collections::HashMap, fs::read_to_string, io, path::PathBuf};
use sxd_document::{parser, writer::format_document};

pub struct SvgRepo {
    base: PathBuf,
    svg_map: HashMap<String, RecordingSurface>,
}

#[derive(Debug, thiserror::Error)]
pub enum SvgRepoError {
    #[error("Error loading SVG ({name}): {source}")]
    LoadingError {
        name: String,
        #[source]
        source: LoadingError,
    },

    #[error("Cairo error ({name}): {source}")]
    CairoError {
        name: String,
        #[source]
        source: cairo::Error,
    },

    #[error("Error rendering SVG ({name}): {source}")]
    RenderingError {
        name: String,
        #[source]
        source: RenderingError,
    },

    #[error("I/O error ({name}): {source}")]
    IoError {
        name: String,
        #[source]
        source: io::Error,
    },

    #[error("XML parsing error ({name}): {source}")]
    XmlParsingError {
        name: String,
        #[source]
        source: sxd_document::parser::Error,
    },
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Options {
    pub name: String,
    pub stylesheet: Option<String>,
    pub halo: bool,
}

impl From<&str> for Options {
    fn from(value: &str) -> Self {
        Self {
            name: value.into(),
            stylesheet: None,
            halo: false,
        }
    }
}

impl SvgRepo {
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

    pub fn get(&mut self, key: &str) -> Result<&RecordingSurface, SvgRepoError> {
        self.get_extra::<fn() -> Options>(key, None)
    }

    pub fn get_extra<T>(
        &mut self,
        key: &str,
        get_options: Option<T>,
    ) -> Result<&RecordingSurface, SvgRepoError>
    where
        T: FnOnce() -> Options,
    {
        let svg_map = &mut self.svg_map;

        if !svg_map.contains_key(key) {
            let options = get_options
                .map(|get_options| get_options())
                .unwrap_or_else(|| Options {
                    name: key.to_string(),
                    stylesheet: None,
                    halo: false,
                });

            let map_loading_error = |err| SvgRepoError::LoadingError {
                name: options.name.clone(),
                source: err,
            };

            let map_cairo_error = |err| SvgRepoError::CairoError {
                name: options.name.clone(),
                source: err,
            };

            let map_io_error = |err| SvgRepoError::IoError {
                name: options.name.clone(),
                source: err,
            };

            let full_path = self.base.join(&options.name);

            let input = read_to_string(full_path).map_err(map_io_error)?;
            let package = parser::parse(&input).map_err(|err| SvgRepoError::XmlParsingError {
                name: options.name.clone(),
                source: err,
            })?;
            let doc = package.as_document();

            if options.halo
                && doc.root().children().len() == 1
                && let Some(svg_element) = doc.root().children()[0].element()
                && svg_element.name().local_part() == "svg"
            {
                let elements = svg_element
                    .children()
                    .iter()
                    .filter_map(|ch| ch.element())
                    .collect::<Vec<_>>();

                if elements.len() == 1 && elements[0].name().local_part() == "path" {
                    elements[0].set_attribute_value(
                        "style",
                        "stroke:#fff;stroke-width:3;stroke-opacity:0.5;stroke-linejoin:round;paint-order:stroke",
                    );
                } else {
                    let u = svg_element.document().create_element("use");

                    u.set_attribute_value("href", "#main");

                    u.set_attribute_value(
                        "style",
                        "stroke:#fff;stroke-width:3;opacity:0.5;stroke-linejoin:round;paint-order:stroke",
                    );

                    svg_element.append_child(u);
                    let g = svg_element.document().create_element("g");

                    g.set_attribute_value("id", "main");

                    svg_element.append_child(g);

                    for e in elements {
                        e.remove_from_parent();
                        g.append_child(e);
                    }
                }
            }

            let mut svg_bytes = Vec::new();
            format_document(&doc, &mut svg_bytes).map_err(map_io_error)?;

            // println!("{}", String::from_utf8(svg_bytes.clone()).unwrap());

            let bytes = glib::Bytes::from_owned(svg_bytes);

            let stream = gio::MemoryInputStream::from_bytes(&bytes);

            let mut handle = rsvg::Loader::new()
                .read_stream(
                    &stream,
                    None::<&gio::File>, // no base file as this document has no references
                    None::<&gio::Cancellable>, // no cancellable
                )
                .map_err(map_loading_error)?;

            if let Some(stylesheet) = options.stylesheet {
                handle
                    .set_stylesheet(&stylesheet)
                    .map_err(map_loading_error)?;
            }

            let renderer = rsvg::CairoRenderer::new(&handle);
            let dim = renderer.intrinsic_size_in_pixels().unwrap_or((16.0, 16.0));
            let rect = Rectangle::new(0.0, 0.0, dim.0, dim.1);
            let surface = RecordingSurface::create(Content::ColorAlpha, Some(rect))
                .map_err(map_cairo_error)?;
            let context = cairo::Context::new(&surface).map_err(map_cairo_error)?;

            renderer.render_document(&context, &rect).map_err(|err| {
                SvgRepoError::RenderingError {
                    name: key.to_string(),
                    source: err,
                }
            })?;

            svg_map.insert(key.to_string(), surface);
        }

        Ok(svg_map.get(key).expect("svg from map"))
    }
}
