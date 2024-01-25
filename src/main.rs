#[macro_use]
extern crate lazy_static;

use crate::routes::RouteTypes;
use cache::Cache;
use cairo::{Context, Format, ImageSurface, Surface, SvgSurface};
use ctx::Ctx;
use gdal::Dataset;
use oxhttp::model::{Request, Response, Status};
use oxhttp::Server;
use postgres::{Config, NoTls};
use r2d2::PooledConnection;
use r2d2_postgres::PostgresConnectionManager;
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;
use xyz::{bbox_size_in_pixels, tile_bounds_to_epsg3857};

mod barrierways;
mod bridge_areas;
mod buildings;
mod cache;
mod colors;
mod contours;
mod ctx;
mod draw;
mod hatcher;
mod hillshading;
mod landuse;
mod line_pattern;
mod power_lines;
mod protected_areas;
mod roads;
mod routes;
mod trees;
mod water_areas;
mod water_lines;
mod xyz;

thread_local! {
    static THREAD_LOCAL_DATA: RefCell<Cache> = {
        let dataset = Dataset::open("/home/martin/14TB/hillshading/sk/final.tif");

        RefCell::new(Cache {
            hillshading_dataset: match dataset {
                Ok(dataset) => Some(dataset),
                _ => {
                    eprintln!("Error opening hillshading geotiff");

                    None
                },
            },
            svg_map: HashMap::new()
        })
    };
}

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

    let mut server = Server::new(move |request| {
        let mut conn = pool.get().unwrap();

        THREAD_LOCAL_DATA.with(|f| render(request, &mut conn, f))
    });

    server.set_num_threads(128);

    // Raise a timeout error if the client does not respond after 10s.
    server.set_global_timeout(Duration::from_secs(10));

    server.listen(("localhost", 3050)).unwrap();
}

fn render<'a>(
    request: &mut Request,
    client: &mut PooledConnection<PostgresConnectionManager<NoTls>>,
    cache: &RefCell<Cache>,
) -> Response {
    let path = request.url().path();

    lazy_static! {
        static ref URL_PATH_REGEXP: Regex =
            Regex::new(r"/(?P<zoom>\d+)/(?P<x>\d+)/(?P<y>\d+)(?:@(?P<scale>\d+(?:\.\d*)?)x)?(?:\.(?P<ext>jpg|png|svg))?")
                .unwrap();
    }

    let x: u32;
    let y: u32;
    let zoom: u32;
    let scale: f64;
    let ext: &str;

    match URL_PATH_REGEXP.captures(path) {
        Some(m) => {
            x = m.name("x").unwrap().as_str().parse::<u32>().unwrap();
            y = m.name("y").unwrap().as_str().parse::<u32>().unwrap();
            zoom = m.name("zoom").unwrap().as_str().parse::<u32>().unwrap();
            scale = m
                .name("scale")
                .map(|m| m.as_str().parse::<f64>().unwrap())
                .unwrap_or(1f64);
            ext = m.name("ext").map(|m| m.as_str()).unwrap_or("png");
        }
        None => {
            return Response::builder(Status::BAD_REQUEST).build();
        }
    }

    let bbox = tile_bounds_to_epsg3857(x, y, zoom, 256);

    let (w, h) = bbox_size_in_pixels(bbox.0, bbox.1, bbox.2, bbox.3, zoom as f64);

    let is_svg = ext == "svg";

    let mut draw = |surface: &Surface| {
        let ctx = Ctx {
            context: Context::new(surface).unwrap(),
            bbox,
            size: (w, h),
            zoom,
            scale,
            cache,
        };

        let context = &ctx.context;

        context.scale(scale, scale);

        context.set_source_rgb(1.0, 1.0, 1.0);

        context.paint().unwrap();

        landuse::render(&ctx, client);

        water_lines::render(&ctx, client);

        water_areas::render(&ctx, client);

        if zoom >= 15 {
            bridge_areas::render(&ctx, client, false);
        }

        if zoom >= 16 {
            trees::render(&ctx, client);
        }

        if zoom >= 8 {
            roads::render(&ctx, client);
        }

        context.push_group();

        if zoom >= 15 {
            bridge_areas::render(&ctx, client, true); // mask
        }

        hillshading::render(&ctx);

        if zoom >= 12 {
            context.push_group();
            contours::render(&ctx, client);
            context.pop_group_to_source().unwrap();
            context.paint().unwrap();
        }

        context.pop_group_to_source().unwrap();
        context.paint().unwrap();

        if zoom >= 13 {
            buildings::render(&ctx, client);
        }

        if zoom >= 16 {
            barrierways::render(&ctx, client);
        }

        if zoom >= 13 {
            power_lines::render_lines(&ctx, client);
        }

        if zoom >= 14 {
            power_lines::render_towers_poles(&ctx, client);
        }

        if zoom >= 12 {
            protected_areas::render(&ctx, client);
        }

        context.save().unwrap();
        routes::render(&ctx, client, &RouteTypes::all());
        context.restore().unwrap();
    };

    let buffer = if is_svg {
        let surface =
            SvgSurface::for_stream(w as f64 * scale, h as f64 * scale, Vec::new()).unwrap();

        draw(surface.deref());

        *surface
            .finish_output_stream()
            .unwrap()
            .downcast::<Vec<u8>>()
            .unwrap()
    } else {
        let mut buffer = Vec::new();

        let surface = ImageSurface::create(
            Format::ARgb32,
            (w as f64 * scale) as i32,
            (h as f64 * scale) as i32,
        )
        .unwrap();

        draw(surface.deref());

        surface.write_to_png(&mut buffer).unwrap();

        buffer
    };

    Response::builder(Status::OK)
        .with_header(
            "Content-Type",
            if is_svg { "image/svg+xml" } else { "image/png" },
        )
        .unwrap()
        .with_body(buffer)
}
