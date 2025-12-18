use crate::layers::routes::RouteTypes;
use crate::layers::{
    blur_edges, embankments, feature_lines_maskable, fixmes, geonames, landcover_names,
    national_park_names, valleys_ridges,
};
use cairo::{
    Content, Context, Format, ImageSurface, PdfSurface, RecordingSurface, Rectangle, Surface,
    SvgSurface,
};
use collision::Collision;
use ctx::Ctx;
use geo::Geometry;
use geo::Rect;
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use layers::{
    aerialway_names, aerialways, aeroways, barrierways, borders, bridge_areas, building_names,
    buildings, country_names, cutlines, feature_lines, features, highway_names, housenumbers,
    landuse, locality_names, military_areas, pipelines, place_names, power_lines,
    protected_area_names, protected_areas, road_access_restrictions, roads, routes, sea,
    shading_and_contours, solar_power_plants, trees, water_area_names, water_areas,
    water_line_names, water_lines,
};
use napi_derive::napi;
use projectable::TileProjector;
use xyz::bbox_size_in_pixels;

pub mod collision;
pub mod colors;
pub mod ctx;
pub mod draw;
pub mod geojson_utils;
pub mod layers;
pub mod projectable;
pub mod re_replacer;
pub mod size;
pub mod svg_cache;
pub mod xyz;
pub use geojson_utils::load_geometry_from_geojson;
pub use layers::hillshading_datasets::{HillshadingDatasets, load_hillshading_datasets};
pub use svg_cache::SvgCache;

pub struct Renderer;

pub fn render(
    request: &RenderRequest,
    client: &mut postgres::Client,
    svg_cache: &mut SvgCache,
    hillshading_datasets: &mut HillshadingDatasets,
    mask_geometry: Option<&Geometry>,
) -> RenderedMap {
    let _span = tracy_client::span!("render_tile");

    let content_type = match request.format {
        ImageFormat::Svg => "image/svg+xml",
        ImageFormat::Pdf => "application/pdf",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
    };

    if request.scales.is_empty() {
        return RenderedMap {
            content_type,
            images: Vec::new(),
        };
    }

    let bbox = request.bbox;

    let size = bbox_size_in_pixels(bbox, request.zoom as f64);

    let scales = request.scales.clone();
    let max_scale = scales
        .iter()
        .copied()
        .fold(1.0_f64, |acc, scale| acc.max(scale));

    let recording_surface = RecordingSurface::create(
        Content::ColorAlpha,
        Some(Rectangle::new(
            0.0,
            0.0,
            size.width as f64,
            size.height as f64,
        )),
    )
    .unwrap();

    {
        let _span = tracy_client::span!("render_tile::draw");

        let hillshade_scale = max_scale.max(1.0);

        draw(
            &recording_surface,
            request,
            client,
            bbox,
            size,
            svg_cache,
            hillshading_datasets,
            hillshade_scale,
            mask_geometry,
        );
    }

    match request.format {
        ImageFormat::Svg => {
            let mut images = Vec::with_capacity(scales.len());

            for scale in scales {
                let w = size.width as f64 * scale;
                let h = size.height as f64 * scale;
                let surface = SvgSurface::for_stream(w, h, Vec::new()).unwrap();

                paint_recording_surface(&recording_surface, &surface, scale);

                let buffer = *surface
                    .finish_output_stream()
                    .unwrap()
                    .downcast::<Vec<u8>>()
                    .unwrap();

                images.push(buffer);
            }

            RenderedMap {
                content_type,
                images,
            }
        }
        ImageFormat::Pdf => {
            let mut images = Vec::with_capacity(scales.len());

            for scale in scales {
                let w = size.width as f64 * scale;
                let h = size.height as f64 * scale;
                let surface = PdfSurface::for_stream(w, h, Vec::new()).unwrap();

                paint_recording_surface(&recording_surface, &surface, scale);

                let buffer = *surface
                    .finish_output_stream()
                    .unwrap()
                    .downcast::<Vec<u8>>()
                    .unwrap();

                images.push(buffer);
            }

            RenderedMap {
                content_type,
                images,
            }
        }
        ImageFormat::Png => {
            let mut images = Vec::with_capacity(scales.len());

            for scale in scales {
                let w = size.width as f64 * scale;
                let h = size.height as f64 * scale;
                let mut buffer = Vec::new();

                let surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32).unwrap();

                paint_recording_surface(&recording_surface, &surface, scale);

                let _span = tracy_client::span!("render_tile::write_to_png");

                surface.write_to_png(&mut buffer).unwrap();

                images.push(buffer);
            }

            RenderedMap {
                content_type,
                images,
            }
        }
        ImageFormat::Jpeg => {
            let mut images = Vec::with_capacity(scales.len());

            for scale in scales {
                let w = size.width as f64 * scale;
                let h = size.height as f64 * scale;
                let mut surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32).unwrap();

                paint_recording_surface(&recording_surface, &surface, scale);

                let width = surface.width() as u32;
                let height = surface.height() as u32;
                let stride = surface.stride() as usize;
                let data = surface.data().unwrap();

                let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);

                for y in 0..height as usize {
                    let row_start = y * stride;
                    let row_end = row_start + width as usize * 4;
                    let row = &data[row_start..row_end];

                    for chunk in row.chunks(4) {
                        let b = chunk[0] as u32;
                        let g = chunk[1] as u32;
                        let r = chunk[2] as u32;
                        let a = chunk[3] as u32;

                        if a == 0 {
                            rgb_data.extend_from_slice(&[0, 0, 0]);
                            continue;
                        }

                        let r = ((r * 255 + a / 2) / a).min(255) as u8;
                        let g = ((g * 255 + a / 2) / a).min(255) as u8;
                        let b = ((b * 255 + a / 2) / a).min(255) as u8;

                        rgb_data.extend_from_slice(&[r, g, b]);
                    }
                }

                let mut buffer = Vec::new();

                {
                    let encoder = JpegEncoder::new_with_quality(&mut buffer, 90);

                    encoder
                        .write_image(&rgb_data, width, height, ExtendedColorType::Rgb8)
                        .unwrap();
                }

                images.push(buffer);
            }

            RenderedMap {
                content_type,
                images,
            }
        }
    }
}

fn draw(
    surface: &Surface,
    request: &RenderRequest,
    client: &mut postgres::Client,
    bbox: Rect<f64>,
    size: crate::size::Size<u32>,
    svg_cache: &mut SvgCache,
    hillshading_datasets: &mut HillshadingDatasets,
    hillshade_scale: f64,
    mask_geometry: Option<&Geometry>,
) {
    let shading = true; // TODO to args
    let contours = true; // TODO to args

    let context = &Context::new(surface).unwrap();

    let collision = &mut Collision::new(Some(context));

    let zoom = request.zoom;

    let ctx = &Ctx {
        context,
        bbox,
        size,
        zoom,
        tile_projector: TileProjector::new(bbox, size),
    };

    sea::render(ctx, client);

    landuse::render(ctx, client, svg_cache);

    if zoom >= 13 {
        cutlines::render(ctx, client);
    }

    water_lines::render(ctx, client, svg_cache);

    water_areas::render(ctx, client);

    if zoom >= 15 {
        bridge_areas::render(ctx, client, false);
    }

    if zoom >= 16 {
        trees::render(ctx, client, svg_cache);
    }

    if zoom >= 12 {
        pipelines::render(ctx, client);
    }

    if zoom >= 13 {
        feature_lines::render(ctx, client, svg_cache);
    }

    if zoom >= 13 {
        feature_lines_maskable::render(
            ctx,
            client,
            svg_cache,
            hillshading_datasets,
            shading,
            hillshade_scale,
        );
    }

    if zoom >= 16 {
        embankments::render(ctx, client, svg_cache);
    }

    if zoom >= 8 {
        roads::render(ctx, client, svg_cache);
    }

    if zoom >= 14 {
        road_access_restrictions::render(ctx, client, svg_cache);
    }

    if shading || contours {
        shading_and_contours::render(
            ctx,
            client,
            hillshading_datasets,
            shading,
            contours,
            hillshade_scale,
        );
    }

    if zoom >= 11 {
        aeroways::render(ctx, client);
    }

    if zoom >= 12 {
        solar_power_plants::render(ctx, client);
    }

    if zoom >= 13 {
        buildings::render(ctx, client);
    }

    if zoom >= 16 {
        barrierways::render(ctx, client);
    }

    if zoom >= 12 {
        aerialways::render(ctx, client);
    }

    if zoom >= 13 {
        power_lines::render_lines(ctx, client);
    }

    if zoom >= 14 {
        power_lines::render_towers_poles(ctx, client);
    }

    if zoom >= 8 {
        protected_areas::render(ctx, client, svg_cache);
    }

    if zoom >= 8 {
        borders::render(ctx, client);
    }

    if zoom >= 10 {
        military_areas::render(ctx, client);
    }

    routes::render_marking(ctx, client, &routes::RouteTypes::all(), svg_cache);

    if (9..=11).contains(&zoom) {
        geonames::render(ctx, client);
    }

    if (8..=14).contains(&zoom) {
        place_names::render(ctx, client, &mut Some(collision));
    }

    if (8..=10).contains(&zoom) {
        national_park_names::render(ctx, client, collision);
    }

    if zoom >= 10 {
        features::render(ctx, client, collision, svg_cache);
    }

    if zoom >= 10 {
        water_area_names::render(ctx, client, collision);
    }

    if zoom >= 17 {
        building_names::render(ctx, client, collision);
    }

    if zoom >= 12 {
        protected_area_names::render(ctx, client, collision);
    }

    if zoom >= 12 {
        landcover_names::render(ctx, client, collision);
    }

    if zoom >= 15 {
        locality_names::render(ctx, client, collision);
    }

    if zoom >= 18 {
        housenumbers::render(ctx, client, collision);
    }

    if zoom >= 15 {
        highway_names::render(ctx, client, collision);
    }

    if zoom >= 14 {
        routes::render_labels(ctx, client, &routes::RouteTypes::all(), collision);
    }

    if zoom >= 16 {
        aerialway_names::render(ctx, client, collision);
    }

    if zoom >= 12 {
        water_line_names::render(ctx, client, collision);
    }

    if zoom >= 14 {
        fixmes::render(ctx, client, svg_cache);
    }

    if zoom >= 13 {
        valleys_ridges::render(ctx, client);
    }

    if zoom >= 15 {
        place_names::render(ctx, client, &mut None);
    }

    if zoom < 8 {
        country_names::render(ctx, client);
    }

    blur_edges::render(ctx, mask_geometry);

    hillshading_datasets.evict_unused();
}

fn paint_recording_surface(
    recording_surface: &RecordingSurface,
    target_surface: &Surface,
    scale: f64,
) {
    let context = Context::new(target_surface).unwrap();
    context.scale(scale, scale);
    context
        .set_source_surface(recording_surface, 0.0, 0.0)
        .unwrap();
    context.paint().unwrap();
}

#[derive(Debug, Clone, Copy)]
#[napi(string_enum)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Pdf,
    Svg,
}

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub bbox: Rect<f64>,
    pub zoom: u32,
    pub scales: Vec<f64>,
    pub format: ImageFormat,
    pub shading: bool,
    pub contours: bool,
    pub route_types: RouteTypes,
}

impl RenderRequest {
    pub fn new(bbox: Rect<f64>, zoom: u32, scales: Vec<f64>, format: ImageFormat) -> Self {
        Self {
            bbox,
            zoom,
            scales,
            format,
            shading: true,
            contours: true,
            route_types: RouteTypes::all(),
        }
    }
}

#[derive(Debug)]
pub struct RenderedMap {
    pub content_type: &'static str,
    pub images: Vec<Vec<u8>>,
}
