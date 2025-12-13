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
    collections::VecDeque,
    net::Ipv4Addr,
    sync::{Arc, Condvar, LazyLock, Mutex, mpsc},
    time::Duration,
};

const SVG_BASE_PATH: &str = "/home/martin/fm/maprender/images";
const HILLSHADING_BASE_PATH: &str = "/home/martin/14TB/hillshading";
const WORKER_COUNT: usize = 24;

struct RenderTask {
    request: RenderRequest,
    resp_tx: mpsc::Sender<Result<maprender_core::RenderedTile, String>>,
}

struct RenderWorkerPool {
    tasks: Arc<Mutex<VecDeque<RenderTask>>>,
    cv: Arc<Condvar>,
}

impl RenderWorkerPool {
    fn new(pool: r2d2::Pool<PostgresConnectionManager<NoTls>>) -> Self {
        let tasks = Arc::new(Mutex::new(VecDeque::new()));
        let cv = Arc::new(Condvar::new());

        for worker_id in 0..WORKER_COUNT {
            let tasks = tasks.clone();
            let cv = cv.clone();
            let pool = pool.clone();

            std::thread::Builder::new()
                .name(format!("render-worker-{worker_id}"))
                .spawn(move || {
                    let mut svg_cache = SvgCache::new(SVG_BASE_PATH);

                    let mut hillshading_datasets = load_hillshading_datasets(HILLSHADING_BASE_PATH);

                    loop {
                        let RenderTask { request, resp_tx } = {
                            let mut guard = tasks.lock().unwrap();
                            while guard.is_empty() {
                                guard = cv.wait(guard).unwrap();
                            }
                            guard.pop_front().unwrap()
                        };

                        let result = match pool.get() {
                            Ok(mut client) => Ok(render_tile(
                                &request,
                                &mut client,
                                &mut svg_cache,
                                &mut hillshading_datasets,
                            )),
                            Err(e) => Err(format!("db pool error: {e}")),
                        };

                        // Ignore send errors (client dropped).
                        let _ = resp_tx.send(result);
                    }
                })
                .expect("render worker spawn");
        }

        Self { tasks, cv }
    }

    fn render(&self, request: RenderRequest) -> Result<maprender_core::RenderedTile, String> {
        let (resp_tx, resp_rx) = mpsc::channel();

        {
            let mut guard = self.tasks.lock().unwrap();
            guard.push_back(RenderTask { request, resp_tx });
            self.cv.notify_one();
        }

        resp_rx.recv().map_err(|e| format!("worker closed: {e}"))?
    }
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

    let connection_pool = r2d2::Pool::builder().max_size(48).build(manager).unwrap();

    let worker_pool = Arc::new(RenderWorkerPool::new(connection_pool.clone()));

    Server::new(move |request| render_response(request, worker_pool.clone()))
        .with_max_concurrent_connections(4096)
        .with_global_timeout(Duration::from_secs(100))
        .bind((Ipv4Addr::LOCALHOST, 3050))
        .spawn()
        .unwrap()
        .join()
        .unwrap();
}

fn render_response(request: &Request<Body>, worker_pool: Arc<RenderWorkerPool>) -> Response<Body> {
    let Some(tile_request) = parse_tile_path(request.uri().path()) else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .expect("body should be built");
    };

    let rendered = match worker_pool.render(tile_request) {
        Ok(rendered) => rendered,
        Err(err) => {
            eprintln!("render failed: {err}");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("render error"))
                .expect("body should be built");
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", rendered.content_type)
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(rendered.buffer))
        .expect("body should be built")
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
