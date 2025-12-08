use crate::{
    collision::Collision,
    layers::{
        aerialway_names, aerialways, aeroways, barrierways, borders, bridge_areas, building_names,
        buildings, cutlines, features,
        highway_names::highway_names,
        housenumbers, landuse, locality_names, military_areas, pipelines, place_names, power_lines,
        protected_area_names, protected_areas, road_access_restrictions, roads, routes,
        shading_and_contours::{self, SHADING_THREAD_LOCAL},
        solar_power_plants, trees, water_area_names, water_areas, water_line_names, water_lines,
    },
    svg_cache::{SVG_CACHE_THREAD_LOCAL, SvgCache},
};
use cairo::{Context, Format, ImageSurface, Surface, SvgSurface};
use ctx::Ctx;
use gdal::Dataset;
use oxhttp::{
    Server,
    model::{Body, Request, Response, StatusCode},
};
use postgres::{Config, NoTls};
use r2d2::PooledConnection;
use r2d2_postgres::PostgresConnectionManager;
use regex::Regex;
use std::{
    cell::RefCell, collections::HashMap, net::Ipv4Addr, ops::Deref, sync::LazyLock, time::Duration,
};
use tracy_client::Client;
use xyz::{bbox_size_in_pixels, tile_bounds_to_epsg3857};

mod bbox;
mod collision;
mod colors;
mod ctx;
mod draw;
mod layers;
mod point;
mod size;
mod svg_cache;
mod xyz;

const SHADING_AND_CONTOURS: bool = true;

pub fn main() {
    Client::start();

    let manager = r2d2_postgres::PostgresConnectionManager::new(
        Config::new()
            .user("martin")
            .password("b0n0")
            .host("localhost")
            .to_owned(),
        NoTls,
    );

    let pool = r2d2::Pool::builder().max_size(24).build(manager).unwrap();

    Server::new(move |request| {
        let mut conn = pool.get().unwrap();

        SHADING_THREAD_LOCAL
            .with(|s| SVG_CACHE_THREAD_LOCAL.with(|f| render(request, &mut conn, f, s)))
    })
    .with_max_concurrent_connections(128)
    .with_global_timeout(Duration::from_secs(10))
    .bind((Ipv4Addr::LOCALHOST, 3050))
    .spawn()
    .unwrap()
    .join()
    .unwrap();
}

fn render(
    request: &Request<Body>,
    client: &mut PooledConnection<PostgresConnectionManager<NoTls>>,
    svg_cache: &RefCell<SvgCache>,
    shading_data: &RefCell<HashMap<String, Dataset>>,
) -> Response<Body> {
    let path = request.uri().path();

    static URL_PATH_REGEXP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"/(?P<zoom>\d+)/(?P<x>\d+)/(?P<y>\d+)(?:@(?P<scale>\d+(?:\.\d*)?)x)?(?:\.(?P<ext>jpg|png|svg))?").unwrap()
    });

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
                .map_or(1f64, |m| m.as_str().parse::<f64>().unwrap());

            ext = m.name("ext").map_or("png", |m| m.as_str());
        }
        None => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .expect("body should be built");
        }
    }

    let bbox = tile_bounds_to_epsg3857(x, y, zoom, 256);

    let is_svg = ext == "svg";

    let size = bbox_size_in_pixels(bbox, zoom as f64);

    let mut draw = |surface: &Surface| {
        let context = Context::new(surface).unwrap();

        // let mut collision = Collision::<f64>::new(Some(&context));
        let mut collision = Collision::<f64>::new(None);

        let ctx = &Ctx {
            context: &context,
            bbox,
            size,
            zoom,
            scale,
            svg_cache,
            shading_data,
        };

        let context = &ctx.context;

        let ctx = &ctx;

        context.scale(scale, scale);

        context.set_source_rgb(1.0, 1.0, 1.0);

        context.paint().unwrap();

        // let path = context.copy_path_flat().unwrap();

        // draw_text_on_path(context, &path, "fimip");

        // TODO sea

        landuse::render(ctx, client);

        if zoom >= 13 {
            cutlines::render(ctx, client);
        }

        water_lines::render(ctx, client);

        water_areas::render(ctx, client);

        if zoom >= 15 {
            bridge_areas::render(ctx, client, false);
        }

        if zoom >= 16 {
            trees::render(ctx, client);
        }

        if zoom >= 12 {
            pipelines::render(ctx, client);
        }

        // TODO feature lines

        // TODO feature lines maskable

        // TODO embankments

        if zoom >= 8 {
            roads::render(ctx, client);
        }

        if zoom >= 14 {
            road_access_restrictions::render(ctx, client);
        }

        if SHADING_AND_CONTOURS {
            shading_and_contours::render(ctx, client);
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
            protected_areas::render(ctx, client);
        }

        borders::render(ctx, client);

        if zoom >= 10 {
            military_areas::render(ctx, client);
        }

        context.save().unwrap();
        routes::render(ctx, client, &routes::RouteTypes::all());
        context.restore().unwrap();

        // TODO geonames

        place_names::render(ctx, client, &mut collision);

        features::render(ctx, client, &mut collision);

        if zoom >= 10 {
            water_area_names::render(ctx, client, &mut collision);
        }

        // TODO national_park_border_names

        if zoom >= 17 {
            building_names::render(ctx, client, &mut collision);
        }

        if zoom >= 12 {
            protected_area_names::render(ctx, client, &mut collision);
        }

        // TODO <LandcoverNames />

        if zoom >= 15 {
            locality_names::render(ctx, client, &mut collision);
        }

        if zoom >= 18 {
            housenumbers::render(ctx, client, &mut collision);
        }

        highway_names(ctx, client, &mut collision);

        // <RouteNames {...routeProps} />

        aerialway_names::render(ctx, client, &mut collision);

        water_line_names::render(ctx, client, &mut collision);

        // <Fixmes />

        // <ValleysRidges />

        // <PlaceNames2 />

        // <CountryNames />
    };

    let w = size.width as f64 * scale;
    let h = size.height as f64 * scale;

    let buffer = if is_svg {
        let surface = SvgSurface::for_stream(w, h, Vec::new()).unwrap();

        draw(surface.deref());

        *surface
            .finish_output_stream()
            .unwrap()
            .downcast::<Vec<u8>>()
            .unwrap()
    } else {
        let mut buffer = Vec::new();

        let surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32).unwrap();

        draw(surface.deref());

        surface.write_to_png(&mut buffer).unwrap();

        buffer
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(
            "Content-Type",
            if is_svg { "image/svg+xml" } else { "image/png" },
        )
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(buffer))
        .expect("body should be built")
}
