use geo::{Geometry, Rect};
use maprender_core::{
    HillshadingDatasets, ImageFormat, RenderRequest, SvgCache, layers::routes::RouteTypes,
    load_geometry_from_geojson, load_hillshading_datasets, render,
};
use napi::bindgen_prelude::*;
use napi::{Error, Result};
use napi_derive::napi;
use postgres::NoTls;

#[napi]
pub struct Renderer {
    client: postgres::Client,
    svg_cache: SvgCache,
    shading_data: Option<HillshadingDatasets>,
    mask_geometry: Option<Geometry>,
}

#[napi(object)]
pub struct RenderResult {
    pub content_type: String,
    pub images: Vec<Buffer>,
}

#[napi(object)]
#[derive(Debug)]
pub struct RequestExtra {
    pub shading: Option<bool>,
    pub contours: Option<bool>,
    pub hiking_routes: Option<bool>,
    pub bicycle_routes: Option<bool>,
    pub ski_routes: Option<bool>,
    pub horse_routes: Option<bool>,
}

#[napi]
impl Renderer {
    #[napi(constructor)]
    pub fn new(
        connection_str: String,
        hillshading_base: Option<String>,
        svg_base: String,
        db_priority: Option<u8>,
        mask_geojson_path: Option<String>,
    ) -> Result<Self> {
        let mut client = postgres::Client::connect(&connection_str, NoTls).map_err(|err| {
            Error::new(
                Status::GenericFailure,
                format!("failed to open postgres connection: {}", err),
            )
        })?;

        if let Some(db_priority) = db_priority {
            client
                .query(
                    &format!("SELECT set_backend_priority(pg_backend_pid(), {db_priority})"),
                    &[],
                )
                .map_err(|err| Error::from_reason(format!("failed to set DB priority: {err}")))?;
        };

        let mask_geometry = mask_geojson_path
            .map(|path| {
                load_geometry_from_geojson(path.as_ref()).map_err(|err| {
                    Error::from_reason(format!("failed to load mask geojson {path}: {err}"))
                })
            })
            .transpose()?;

        Ok(Self {
            svg_cache: SvgCache::new(svg_base),
            shading_data: hillshading_base.map(load_hillshading_datasets),
            client,
            mask_geometry,
        })
    }

    #[napi]
    pub fn render(
        &mut self,
        bbox: (f64, f64, f64, f64),
        zoom: u32,
        scales: Vec<f64>,
        format: ImageFormat,
        extra: Option<RequestExtra>,
    ) -> Result<RenderResult> {
        let bbox = Rect::new((bbox.0, bbox.1), (bbox.2, bbox.3));

        let mut request = RenderRequest::new(bbox, zoom, scales, format);

        if let Some(extra) = extra {
            request.shading = extra.shading.unwrap_or(true);
            request.contours = extra.contours.unwrap_or(true);

            if extra.hiking_routes.is_some()
                && extra.bicycle_routes.is_some()
                && extra.ski_routes.is_some()
                && extra.horse_routes.is_some()
            {
                let mut route_types = RouteTypes::empty();

                route_types.set(RouteTypes::HIKING, extra.hiking_routes.unwrap_or(true));
                route_types.set(RouteTypes::BICYCLE, extra.bicycle_routes.unwrap_or(true));
                route_types.set(RouteTypes::SKI, extra.ski_routes.unwrap_or(true));
                route_types.set(RouteTypes::HORSE, extra.horse_routes.unwrap_or(true));

                request.route_types = route_types;
            }
        }

        let rendered = render(
            &request,
            &mut self.client,
            &mut self.svg_cache,
            &mut self.shading_data,
            self.mask_geometry.as_ref(),
        );

        Ok(RenderResult {
            content_type: rendered.content_type.to_string(),
            images: rendered.images.into_iter().map(Buffer::from).collect(),
        })
    }
}
