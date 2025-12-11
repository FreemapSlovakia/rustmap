use geo::Rect;
use maprender_core::{RenderRequest, TileFormat};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use postgres::NoTls;

#[napi]
pub mod bindings {
    use std::collections::HashMap;

    use super::*;
    use gdal::Dataset;
    use maprender_core::{SvgCache, load_hillshading_datasets, render_tile};

    #[napi]
    pub struct Renderer {
        client: postgres::Client,
        svg_cache: SvgCache,
        shading_data: HashMap<String, Dataset>,
    }

    #[napi(object)]
    pub struct RenderResult {
        pub data: Buffer,
        pub content_type: String,
    }

    #[napi]
    impl Renderer {
        #[napi(constructor)]
        pub fn new(
            connection_str: String,
            hillshading_base: String,
            svg_base: String,
        ) -> Result<Self> {
            let client = postgres::Client::connect(&connection_str, NoTls).map_err(|err| {
                Error::new(
                    Status::GenericFailure,
                    format!("failed to open postgres connection: {}", err),
                )
            })?;

            Ok(Self {
                svg_cache: SvgCache::new(svg_base),
                shading_data: load_hillshading_datasets(hillshading_base),
                client,
            })
        }

        #[napi]
        pub fn render(
            &mut self,
            bbox: (f64, f64, f64, f64),
            zoom: u32,
            scale: Option<f64>,
            format: Option<String>,
        ) -> Result<RenderResult> {
            let request = build_request(bbox, zoom, scale, format)?;

            let rendered = render_tile(
                &request,
                &mut self.client,
                &mut self.svg_cache,
                &mut self.shading_data,
            );

            Ok(RenderResult {
                data: Buffer::from(rendered.buffer),
                content_type: rendered.content_type.to_string(),
            })
        }
    }

    fn build_request(
        bbox: (f64, f64, f64, f64),
        zoom: u32,
        scale: Option<f64>,
        format: Option<String>,
    ) -> Result<RenderRequest> {
        let bbox = Rect::new((bbox.0, bbox.1), (bbox.2, bbox.3));

        let format = match format.as_deref() {
            Some("svg") => TileFormat::Svg,
            Some("pdf") => TileFormat::Pdf,
            Some("jpg" | "jpeg") => TileFormat::Jpeg,
            Some("png") | None => TileFormat::Png,
            Some(other) => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!("unsupported format {}", other),
                ));
            }
        };

        Ok(RenderRequest::new(bbox, zoom, scale.unwrap_or(1.0), format))
    }
}
