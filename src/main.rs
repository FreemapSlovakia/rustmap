#[macro_use]
extern crate lazy_static;

use cache::Cache;
use cairo::{Context, Format, ImageSurface};
use ctx::Ctx;
use gdal::Dataset;
use lockfree_object_pool::MutexObjectPool;
use oxhttp::model::{Request, Response, Status};
use oxhttp::Server;
use postgres::{Config, NoTls};
use r2d2::PooledConnection;
use r2d2_postgres::PostgresConnectionManager;
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use xyz::{bbox_size_in_pixels, tile_bounds_to_epsg3857};

mod buildings;
mod cache;
mod colors;
mod contours;
mod ctx;
mod draw;
mod hillshading;
mod landuse;
mod pois;
mod roads;
mod water_areas;
mod water_lines;
mod xyz;

pub fn main() {
    let manager = r2d2_postgres::PostgresConnectionManager::new(
        Config::new()
            .user("martin")
            .password("b0n0")
            .host("localhost")
            .to_owned(),
        NoTls,
    );

    let pool = r2d2::Pool::builder().max_size(24).build(manager).unwrap();

    let object_pool = Arc::new(MutexObjectPool::new(
        || Cache {
            hillshading_dataset: Dataset::open("/home/martin/14TB/hillshading/sk/final.tif")
                .unwrap(),
            // svg_map: HashMap::new(),
        },
        |_| {},
    ));

    let mut server = Server::new(move |request| {
        let mut conn = pool.get().unwrap();

        let object_pool = object_pool.clone();

        let mut cache = object_pool.pull();

        render(request, &mut conn, &mut cache)
    });

    server.set_num_threads(128);

    // Raise a timeout error if the client does not respond after 10s.
    server.set_global_timeout(Duration::from_secs(10));

    server.listen(("localhost", 3050)).unwrap();
}

fn render<'a>(
    request: &mut Request,
    client: &mut PooledConnection<PostgresConnectionManager<NoTls>>,
    cache: &mut Cache,
) -> Response {
    let path = request.url().path();

    lazy_static! {
        static ref URL_PATH_REGEXP: Regex =
            Regex::new(r"/(?P<zoom>\d+)/(?P<x>\d+)/(?P<y>\d+)(?:@(?P<scale>\d+(?:\.\d*)?)x)?")
                .unwrap();
    }

    let x: u32;
    let y: u32;
    let zoom: u32;
    let scale: f64;

    match URL_PATH_REGEXP.captures(path) {
        Some(m) => {
            x = m.name("x").unwrap().as_str().parse::<u32>().unwrap();
            y = m.name("y").unwrap().as_str().parse::<u32>().unwrap();
            zoom = m.name("zoom").unwrap().as_str().parse::<u32>().unwrap();
            scale = m
                .name("scale")
                .map(|m| m.as_str().parse::<f64>().unwrap())
                .unwrap_or_else(|| 1f64);
        }
        None => {
            return Response::builder(Status::BAD_REQUEST).build();
        }
    }

    let bbox = tile_bounds_to_epsg3857(x, y, zoom, 256);

    let (w, h) = bbox_size_in_pixels(bbox.0, bbox.1, bbox.2, bbox.3, zoom as f64);

    let surface = ImageSurface::create(
        Format::ARgb32,
        (w as f64 * scale) as i32,
        (h as f64 * scale) as i32,
    )
    .unwrap();

    let ctx = Ctx {
        context: Context::new(&surface).unwrap(),
        bbox,
        size: (w, h),
        zoom,
        cache,
    };

    let context = &ctx.context;

    context.scale(scale, scale);

    context.set_source_rgb(1.0, 1.0, 1.0);

    context.paint().unwrap();

    landuse::render(&ctx, client);

    // TODO cutlines

    water_lines::render(&ctx, client);

    water_areas::render(&ctx, client);

    roads::render(&ctx, client);

    hillshading::render(&ctx, scale);

    if zoom > 11 {
        contours::render(&ctx, client);
    }

    if zoom > 12 {
        buildings::render(&ctx, client);
    }

    // pois::render(context);

    // context.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);

    // context.set_font_size(40.0);

    // context.move_to(100., 140.);

    // context.text_path("Hello, Cairo!");

    // context.set_source_rgb(0.0, 0.0, 1.0); // Blue color for fill
    // context.fill_preserve().unwrap();

    // context.set_source_rgba(1.0, 1.0, 1.0, 0.75); // Red color for stroke
    // context.set_line_width(10.0);
    // context.set_line_join(cairo::LineJoin::Round);
    // context.stroke().unwrap();

    // context.move_to(100., 140.);
    // context.set_source_rgb(0.0, 0.0, 0.0); // Black color
    // context.show_text("Hello, Cairo!").unwrap();

    // debug tile border
    // context.set_dash(&[], 0.0);
    // context.set_line_width(2.0);
    // context.set_source_rgb(0.0, 0.0, 1.0); // Red border for visibility
    // context.rectangle(0.0, 0.0, w as f64, h as f64);
    // context.stroke().unwrap();

    let mut buffer = Vec::new();

    surface.write_to_png(&mut buffer).unwrap();

    Response::builder(Status::OK)
        .with_header("Content-Type", "image/png")
        .unwrap()
        .with_body(buffer)
}
