use clap::Parser;
use dotenvy::dotenv;
use geo::Geometry;
use maprender_core::{
    ImageFormat, RenderError, RenderRequest, SvgRepo, load_geometry_from_geojson,
    load_hillshading_datasets, render, tile_bounds_to_epsg3857,
};
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
    sync::{
        Arc, Condvar, LazyLock, Mutex,
        mpsc::{self, RecvError},
    },
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

    /// Mask geojson polygon file
    #[arg(long, env = "MAPRENDER_MASK_GEOJSON")]
    mask_geojson: Option<String>,
}

struct RenderTask {
    request: RenderRequest,
    resp_tx: mpsc::Sender<Result<Vec<Vec<u8>>, ReError>>,
}

struct RenderWorkerPool {
    tasks: Arc<Mutex<VecDeque<RenderTask>>>,
    cv: Arc<Condvar>,
}

#[derive(Debug, thiserror::Error)]
enum ReError {
    #[error(transparent)]
    RenderError(#[from] RenderError),

    #[error(transparent)]
    ConnectionPoolError(#[from] r2d2::Error),

    #[error("worker closed: {0}")]
    RecvError(#[from] RecvError),
}

impl RenderWorkerPool {
    fn new(
        pool: r2d2::Pool<PostgresConnectionManager<NoTls>>,
        worker_count: usize,
        svg_base_path: Arc<str>,
        hillshading_base_path: Arc<str>,
        mask_geometry: Option<Geometry>,
    ) -> Self {
        let tasks = Arc::new(Mutex::new(VecDeque::new()));
        let cv = Arc::new(Condvar::new());

        for worker_id in 0..worker_count {
            let tasks = tasks.clone();
            let cv = cv.clone();
            let pool = pool.clone();
            let svg_base_path = svg_base_path.clone();
            let hillshading_base_path = hillshading_base_path.clone();
            let mask_geometry = mask_geometry.clone();

            std::thread::Builder::new()
                .name(format!("render-worker-{worker_id}"))
                .spawn(move || {
                    let mut svg_repo = SvgRepo::new(&*svg_base_path);

                    let mut hillshading_datasets =
                        Some(load_hillshading_datasets(&*hillshading_base_path));

                    loop {
                        let RenderTask { request, resp_tx } = {
                            let mut guard = tasks.lock().unwrap();
                            while guard.is_empty() {
                                guard = cv.wait(guard).unwrap();
                            }
                            guard.pop_front().unwrap()
                        };

                        let result = pool.get().map_err(ReError::from).and_then(|mut client| {
                            render(
                                &request,
                                &mut client,
                                &mut svg_repo,
                                &mut hillshading_datasets,
                                mask_geometry.as_ref(),
                            )
                            .map_err(ReError::from)
                        });

                        // Ignore send errors (client dropped).
                        let _ = resp_tx.send(result);
                    }
                })
                .expect("render worker spawn");
        }

        Self { tasks, cv }
    }

    fn render(&self, request: RenderRequest) -> Result<Vec<Vec<u8>>, ReError> {
        let (resp_tx, resp_rx) = mpsc::channel();

        {
            let mut guard = self.tasks.lock().unwrap();
            guard.push_back(RenderTask { request, resp_tx });
            self.cv.notify_one();
        }

        resp_rx.recv()?
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

    let mask_geometry =
        cli.mask_geojson
            .map(|path| match load_geometry_from_geojson(path.as_ref()) {
                Ok(g) => g,
                Err(err) => panic!("failed to load mask geojson {path}: {err}"),
            });

    let worker_pool = Arc::new(RenderWorkerPool::new(
        connection_pool,
        cli.worker_count,
        Arc::from(cli.svg_base_path.as_str()),
        Arc::from(cli.hillshading_base_path.as_str()),
        mask_geometry,
    ));

    Server::new(move |request| render_response(request, worker_pool.clone()))
        .with_max_concurrent_connections(cli.max_concurrent_connections)
        .with_global_timeout(Duration::from_secs(cli.global_timeout_secs))
        .bind((cli.host, cli.port))
        .spawn()
        .expect("server spawned")
        .join()
        .expect("server joined");
}

fn render_response(request: &Request<Body>, worker_pool: Arc<RenderWorkerPool>) -> Response<Body> {
    let Some(tile_request) = parse_tile_path(request.uri().path()) else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::empty())
            .expect("body should be built");
    };

    let format = tile_request.format;

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

    if let Some(tile) = rendered.into_iter().next() {
        let content_type = match format {
            ImageFormat::Svg => "image/svg+xml",
            ImageFormat::Pdf => "application/pdf",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
        };
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", content_type)
            .header("Access-Control-Allow-Origin", "*")
            .body(Body::from(tile))
    } else {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("empty render result"))
    }
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
        "svg" => ImageFormat::Svg,
        "pdf" => ImageFormat::Pdf,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        _ => ImageFormat::Png,
    };

    let bbox = tile_bounds_to_epsg3857(x, y, zoom, 256);

    Some(RenderRequest::new(bbox, zoom, vec![scale], format))
}
