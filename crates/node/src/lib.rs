use std::collections::HashMap;

use gdal::Dataset;
use geo::Rect;
use maprender_core::{RenderRequest, SvgCache, TileFormat, load_hillshading_datasets, render};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use postgres::NoTls;

#[napi]
pub struct Renderer {
    client: postgres::Client,
    svg_cache: SvgCache,
    shading_data: HashMap<String, Dataset>,
}

#[napi(object)]
pub struct RenderResult {
    pub content_type: String,
    pub images: Vec<Buffer>,
}

#[napi]
impl Renderer {
    #[napi(constructor)]
    pub fn new(connection_str: String, hillshading_base: String, svg_base: String) -> Result<Self> {
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
        scales: Vec<f64>,
        format: Option<String>,
    ) -> Result<RenderResult> {
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

        let rendered = render(
            &RenderRequest::new(bbox, zoom, scales, format),
            &mut self.client,
            &mut self.svg_cache,
            &mut self.shading_data,
        );

        Ok(RenderResult {
            content_type: rendered.content_type.to_string(),
            images: rendered.images.into_iter().map(Buffer::from).collect(),
        })
    }
}
