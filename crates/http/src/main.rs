use clap::Parser;
use dotenvy::dotenv;
use maprender_core::xyz::tile_bounds_to_epsg3857;
use maprender_core::{RenderRequest, SvgCache, TileFormat, load_hillshading_datasets, render};
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
    str::FromStr,
    sync::{Arc, Condvar, LazyLock, Mutex, mpsc},
    time::Duration,
};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Path to the directory with symbol SVGs.
    #[arg(long, env = "MAPRENDER_SVG_BASE_PATH")]
    svg_base_path: String,

    /// Path to hillshading datasets.
    #[arg(long, env = "MAPRENDER_HILLSHADING_BASE_PATH")]
    hillshading_base_path: String,

    /// Number of rendering worker threads.
    #[arg(long, env = "MAPRENDER_WORKER_COUNT", default_value_t = 24)]
    worker_count: usize,

    /// Database connection string (e.g. postgres://user:pass@host/dbname).
    #[arg(long, env = "MAPRENDER_DATABASE_URL")]
    database_url: String,

    /// HTTP bind address.
    #[arg(long, env = "MAPRENDER_HOST", default_value_t = Ipv4Addr::LOCALHOST)]
    host: Ipv4Addr,

    /// HTTP bind port.
    #[arg(long, env = "MAPRENDER_PORT", default_value_t = 3050)]
    port: u16,

    /// Maximum concurrent HTTP connections.
    #[arg(
        long,
        env = "MAPRENDER_MAX_CONCURRENT_CONNECTIONS",
        default_value_t = 4096
    )]
    max_concurrent_connections: usize,

    /// Global HTTP timeout in seconds.
    #[arg(long, env = "MAPRENDER_GLOBAL_TIMEOUT_SECS", default_value_t = 100)]
    global_timeout_secs: u64,

    /// Database pool max size.
    #[arg(long, env = "MAPRENDER_POOL_MAX_SIZE", default_value_t = 48)]
    pool_max_size: u32,
}

struct RenderTask {
    request: RenderRequest,
    resp_tx: mpsc::Sender<Result<maprender_core::RenderedMap, String>>,
}

struct RenderWorkerPool {
    tasks: Arc<Mutex<VecDeque<RenderTask>>>,
    cv: Arc<Condvar>,
}

impl RenderWorkerPool {
    fn new(
        pool: r2d2::Pool<PostgresConnectionManager<NoTls>>,
        worker_count: usize,
        svg_base_path: Arc<str>,
        hillshading_base_path: Arc<str>,
    ) -> Self {
        let tasks = Arc::new(Mutex::new(VecDeque::new()));
        let cv = Arc::new(Condvar::new());

        for worker_id in 0..worker_count {
            let tasks = tasks.clone();
            let cv = cv.clone();
            let pool = pool.clone();
            let svg_base_path = svg_base_path.clone();
            let hillshading_base_path = hillshading_base_path.clone();

            std::thread::Builder::new()
                .name(format!("render-worker-{worker_id}"))
                .spawn(move || {
                    let mut svg_cache = SvgCache::new(&*svg_base_path);

                    let mut hillshading_datasets =
                        load_hillshading_datasets(&*hillshading_base_path);

                    loop {
                        let RenderTask { request, resp_tx } = {
                            let mut guard = tasks.lock().unwrap();
                            while guard.is_empty() {
                                guard = cv.wait(guard).unwrap();
                            }
                            guard.pop_front().unwrap()
                        };

                        let result = match pool.get() {
                            Ok(mut client) => Ok(render(
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

    fn render(&self, request: RenderRequest) -> Result<maprender_core::RenderedMap, String> {
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
    dotenv().ok();

    tracy_client::Client::start();

    let cli = Cli::parse();

    let pg_config = Config::from_str(&cli.database_url).expect("parse database url");

    let manager = PostgresConnectionManager::new(pg_config, NoTls);

    let connection_pool = r2d2::Pool::builder()
        .max_size(cli.pool_max_size)
        .build(manager)
        .expect("build db pool");

    let worker_pool = Arc::new(RenderWorkerPool::new(
        connection_pool.clone(),
        cli.worker_count,
        Arc::from(cli.svg_base_path.as_str()),
        Arc::from(cli.hillshading_base_path.as_str()),
    ));

    Server::new(move |request| render_response(request, worker_pool.clone()))
        .with_max_concurrent_connections(cli.max_concurrent_connections)
        .with_global_timeout(Duration::from_secs(cli.global_timeout_secs))
        .bind((cli.host, cli.port))
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

    let content_type = rendered.content_type;
    let Some(tile) = rendered.images.into_iter().next() else {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("empty render result"))
            .expect("body should be built");
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(tile))
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

    Some(RenderRequest::new_single(bbox, zoom, scale, format))
}
