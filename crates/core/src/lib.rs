use cairo::{Context, Format, ImageSurface, PdfSurface, Surface, SvgSurface};
use collision::Collision;
use colors::ContextExt;
use ctx::Ctx;
use gdal::Dataset;
use geo::Rect;
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use layers::{
    aerialway_names, aerialways, aeroways, barrierways, borders, bridge_areas, building_names,
    buildings, country_names, cutlines, feature_lines, features, highway_names::highway_names,
    housenumbers, landuse, locality_names, military_areas, pipelines, place_names, power_lines,
    protected_area_names, protected_areas, road_access_restrictions, roads, routes, sea,
    shading_and_contours, solar_power_plants, trees, water_area_names, water_areas,
    water_line_names, water_lines,
};
use projectable::TileProjector;
use std::collections::HashMap;
use xyz::bbox_size_in_pixels;

pub mod collision;
pub mod colors;
pub mod ctx;
pub mod draw;
pub mod layers;
pub mod projectable;
pub mod size;
pub mod svg_cache;
pub mod xyz;

pub use shading_and_contours::load_hillshading_datasets;
pub use svg_cache::SvgCache;

pub struct Renderer;

pub fn render_tile(
    request: &RenderRequest,
    client: &mut postgres::Client,
    svg_cache: &mut SvgCache,
    hillshading_datasets: &mut HashMap<String, Dataset>,
) -> RenderedTile {
    let bbox = request.bbox;

    let size = bbox_size_in_pixels(bbox, request.zoom as f64);

    let mut draw = |surface: &Surface| {
        draw(
            surface,
            request,
            client,
            bbox,
            size,
            svg_cache,
            hillshading_datasets,
        );
    };

    let w = size.width as f64 * request.scale;
    let h = size.height as f64 * request.scale;

    match request.format {
        TileFormat::Svg => {
            let surface = SvgSurface::for_stream(w, h, Vec::new()).unwrap();

            draw(&surface);

            let buffer = *surface
                .finish_output_stream()
                .unwrap()
                .downcast::<Vec<u8>>()
                .unwrap();

            RenderedTile {
                content_type: "image/svg+xml",
                buffer,
            }
        }
        TileFormat::Pdf => {
            let surface = PdfSurface::for_stream(w, h, Vec::new()).unwrap();

            draw(&surface);

            let buffer = *surface
                .finish_output_stream()
                .unwrap()
                .downcast::<Vec<u8>>()
                .unwrap();

            RenderedTile {
                content_type: "application/pdf",
                buffer,
            }
        }
        TileFormat::Png => {
            let mut buffer = Vec::new();

            let surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32).unwrap();

            draw(&surface);

            surface.write_to_png(&mut buffer).unwrap();

            RenderedTile {
                content_type: "image/png",
                buffer,
            }
        }
        TileFormat::Jpeg => {
            let mut surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32).unwrap();

            draw(&surface);

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

            let encoder = JpegEncoder::new_with_quality(&mut buffer, 90);

            encoder
                .write_image(&rgb_data, width, height, ExtendedColorType::Rgb8)
                .unwrap();

            RenderedTile {
                content_type: "image/jpeg",
                buffer,
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
    hillshading_datasets: &mut HashMap<String, Dataset>,
) {
    let context = Context::new(surface).unwrap();

    // let mut collision = Collision::<f64>::new(Some(&context));
    let mut collision = Collision::<f64>::new(None);

    let ctx = &Ctx {
        context: &context,
        bbox,
        size,
        zoom: request.zoom,
        scale: request.scale,
        tile_projector: TileProjector::new(bbox, size),
    };

    let context = &ctx.context;

    context.scale(request.scale, request.scale);

    context.save().unwrap();
    context.set_source_color(colors::WATER);
    context.paint().unwrap();
    context.restore().unwrap();

    sea::render(ctx, client);

    landuse::render(ctx, client, svg_cache);

    if request.zoom >= 13 {
        cutlines::render(ctx, client);
    }

    water_lines::render(ctx, client, svg_cache);

    water_areas::render(ctx, client);

    if request.zoom >= 15 {
        bridge_areas::render(ctx, client, false);
    }

    if request.zoom >= 16 {
        trees::render(ctx, client, svg_cache);
    }

    if request.zoom >= 12 {
        pipelines::render(ctx, client);
    }

    if request.zoom >= 13 {
        feature_lines::render(ctx, client, svg_cache);
    }

    // TODO feature lines maskable

    // TODO embankments

    if request.zoom >= 8 {
        roads::render(ctx, client, svg_cache);
    }

    if request.zoom >= 14 {
        road_access_restrictions::render(ctx, client, svg_cache);
    }

    if SHADING_AND_CONTOURS {
        shading_and_contours::render(ctx, client, hillshading_datasets);
    }

    if request.zoom >= 11 {
        aeroways::render(ctx, client);
    }

    if request.zoom >= 12 {
        solar_power_plants::render(ctx, client);
    }

    if request.zoom >= 13 {
        buildings::render(ctx, client);
    }

    if request.zoom >= 16 {
        barrierways::render(ctx, client);
    }

    if request.zoom >= 12 {
        aerialways::render(ctx, client);
    }

    if request.zoom >= 13 {
        power_lines::render_lines(ctx, client);
    }

    if request.zoom >= 14 {
        power_lines::render_towers_poles(ctx, client);
    }

    if request.zoom >= 8 {
        protected_areas::render(ctx, client, svg_cache);
    }

    if request.zoom >= 8 {
        borders::render(ctx, client);
    }

    if request.zoom >= 10 {
        military_areas::render(ctx, client);
    }

    context.save().unwrap();
    routes::render(ctx, client, &routes::RouteTypes::all(), svg_cache);
    context.restore().unwrap();

    // TODO geonames

    if request.zoom >= 8 {
        place_names::render(ctx, client, &mut collision);
    }

    // <NationalParkNames />

    features::render(ctx, client, &mut collision, svg_cache);

    if request.zoom >= 10 {
        water_area_names::render(ctx, client, &mut collision);
    }

    if request.zoom >= 17 {
        building_names::render(ctx, client, &mut collision);
    }

    if request.zoom >= 12 {
        protected_area_names::render(ctx, client, &mut collision);
    }

    // TODO <LandcoverNames />

    if request.zoom >= 15 {
        locality_names::render(ctx, client, &mut collision);
    }

    if request.zoom >= 18 {
        housenumbers::render(ctx, client, &mut collision);
    }

    if request.zoom >= 15 {
        highway_names(ctx, client, &mut collision);
    }

    // <RouteNames {...routeProps} />

    if request.zoom >= 16 {
        aerialway_names::render(ctx, client, &mut collision);
    }

    if request.zoom >= 12 {
        water_line_names::render(ctx, client, &mut collision);
    }

    // <Fixmes />

    // <ValleysRidges />

    // <PlaceNames2 />

    if request.zoom < 8 {
        country_names::render(ctx, client);
    }
}

pub const SHADING_AND_CONTOURS: bool = true;

#[derive(Debug, Clone, Copy)]
pub enum TileFormat {
    Png,
    Jpeg,
    Pdf,
    Svg,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderRequest {
    pub bbox: Rect<f64>,
    pub zoom: u32,
    pub scale: f64,
    pub format: TileFormat,
}

impl RenderRequest {
    pub fn new(bbox: Rect<f64>, zoom: u32, scale: f64, format: TileFormat) -> Self {
        Self {
            bbox,
            zoom,
            scale,
            format,
        }
    }
}

#[derive(Debug)]
pub struct RenderedTile {
    pub content_type: &'static str,
    pub buffer: Vec<u8>,
}
