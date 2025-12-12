use gdal::Dataset;
use maprender_core::xyz::tile_bounds_to_epsg3857;
use maprender_core::{RenderRequest, SvgCache, TileFormat, load_hillshading_datasets, render_tile};
use oxhttp::{
    Server,
    model::{Body, Request, Response, StatusCode},
};
use postgres::{Config, NoTls};
use r2d2_postgres::PostgresConnectionManager;
use regex::Regex;
use std::{
    cell::{LazyCell, RefCell},
    collections::HashMap,
    net::Ipv4Addr,
    sync::LazyLock,
    time::Duration,
};

const SVG_BASE_PATH: &str = "/home/martin/fm/maprender/images";
const HILLSHADING_BASE_PATH: &str = "/home/martin/14TB/hillshading";

struct RenderResources {
    svg_cache: SvgCache,
    hillshading_datasets: HashMap<String, Dataset>,
}

thread_local! {
    static RENDER_RESOURCES: LazyCell<RefCell<RenderResources>> = const {
        LazyCell::new(|| RefCell::new(RenderResources {
            svg_cache: SvgCache::new(SVG_BASE_PATH),
            hillshading_datasets: load_hillshading_datasets(HILLSHADING_BASE_PATH),
        }))
    };
}

fn main() {
    tracy_client::Client::start();

    let manager = PostgresConnectionManager::new(
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
        render_response(request, &mut conn)
    })
    .with_max_concurrent_connections(128)
    .with_global_timeout(Duration::from_secs(10))
    .bind((Ipv4Addr::LOCALHOST, 3050))
    .spawn()
    .unwrap()
    .join()
    .unwrap();
}

fn render_response(request: &Request<Body>, client: &mut postgres::Client) -> Response<Body> {
    let Some(tile_request) = parse_tile_path(request.uri().path()) else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .expect("body should be built");
    };

    RENDER_RESOURCES.with(|slot| {
        let mut resources = slot.borrow_mut();

        let RenderResources {
            svg_cache,
            hillshading_datasets,
        } = &mut *resources;

        let rendered = render_tile(&tile_request, client, svg_cache, hillshading_datasets);

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", rendered.content_type)
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::from(rendered.buffer))
            .expect("body should be built")
    })
}

fn parse_tile_path(path: &str) -> Option<RenderRequest> {
    static URL_PATH_REGEXP: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"/(?P<zoom>\d+)/(?P<x>\d+)/(?P<y>\d+)(?:@(?P<scale>\d+(?:\.\d*)?)x)?(?:\.(?P<ext>jpg|jpeg|png|svg|pdf))?")
            .unwrap()
    });

    let captures = URL_PATH_REGEXP.captures(path)?;

    let x = captures
        .name("x")
        .and_then(|m| m.as_str().parse::<u32>().ok())?;

    let y = captures
        .name("y")
        .and_then(|m| m.as_str().parse::<u32>().ok())?;

    let zoom = captures
        .name("zoom")
        .and_then(|m| m.as_str().parse::<u32>().ok())?;

    let scale = captures
        .name("scale")
        .and_then(|m| m.as_str().parse::<f64>().ok())
        .unwrap_or(1.0);

    let ext = captures.name("ext").map(|m| m.as_str()).unwrap_or("png");

    let format = match ext {
        "svg" => TileFormat::Svg,
        "pdf" => TileFormat::Pdf,
        "jpg" | "jpeg" => TileFormat::Jpeg,
        _ => TileFormat::Png,
    };

    let bbox = tile_bounds_to_epsg3857(x, y, zoom, 256);

    Some(RenderRequest::new(bbox, zoom, scale, format))
}
