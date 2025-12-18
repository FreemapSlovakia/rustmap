use crate::{
    ctx::Ctx,
    layers::{bridge_areas, contours, hillshading},
};
use gdal::Dataset;
use postgres::Client;
use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const FALLBACK: bool = true;

const DATASET_PATHS: [(&str, &str); 17] = [
    ("sk", "sk/final.tif"),
    ("sk-mask", "sk/mask.tif"),
    ("cz", "cz/final.tif"),
    ("cz-mask", "cz/mask.tif"),
    ("at", "at/final.tif"),
    ("at-mask", "at/mask.tif"),
    ("pl", "pl/final.tif"),
    ("pl-mask", "pl/mask.tif"),
    ("it", "it/final.tif"),
    ("it-mask", "it/mask.tif"),
    ("ch", "ch/final.tif"),
    ("ch-mask", "ch/mask.tif"),
    ("si", "si/final.tif"),
    ("si-mask", "si/mask.tif"),
    ("fr", "fr/final.tif"),
    ("fr-mask", "fr/mask.tif"),
    ("_", "final.tiff"),
];

const EVICT_AFTER: Duration = Duration::from_secs(10);

struct CachedDataset {
    dataset: Dataset,
    last_used: Instant,
}

pub struct HillshadingDatasets {
    base: PathBuf,
    datasets: HashMap<String, CachedDataset>,
}

impl HillshadingDatasets {
    pub fn new(base: impl AsRef<Path>) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            datasets: HashMap::new(),
        }
    }

    pub fn evict_unused(&mut self) {
        let now = Instant::now();

        self.datasets
            .retain(|_, cached| now.duration_since(cached.last_used) <= EVICT_AFTER);
    }

    pub fn get(&mut self, name: &str) -> Option<&Dataset> {
        let now = Instant::now();

        self.evict_unused();

        match self.datasets.entry(name.to_string()) {
            Entry::Occupied(occ) => {
                let entry = occ.into_mut();

                entry.last_used = now;

                Some(&entry.dataset)
            }
            Entry::Vacant(vac) => {
                let Some(path) = dataset_path(name) else {
                    eprintln!("Unknown hillshading dataset key: {name}");
                    return None;
                };

                let full_path = self.base.join(path);

                match Dataset::open(&full_path) {
                    Ok(dataset) => {
                        let entry = vac.insert(CachedDataset {
                            dataset,
                            last_used: now,
                        });

                        Some(&entry.dataset)
                    }
                    Err(err) => {
                        eprintln!(
                            "Error opening hillshading geotiff {}: {}",
                            full_path.display(),
                            err
                        );

                        None
                    }
                }
            }
        }
    }
}

fn dataset_path(name: &str) -> Option<&'static str> {
    DATASET_PATHS
        .iter()
        .find(|(dataset_name, _)| dataset_name == &name)
        .map(|(_, path)| *path)
}

pub fn load_hillshading_datasets(base: impl AsRef<Path>) -> HillshadingDatasets {
    HillshadingDatasets::new(base)
}

pub fn render(
    ctx: &Ctx,
    client: &mut Client,
    hillshading_datasets: &mut HillshadingDatasets,
    shading: bool,
    contours: bool,
    hillshade_scale: f64,
) {
    let _span = tracy_client::span!("shading_and_contours::render");

    hillshading_datasets.evict_unused();

    let fade_alpha = 1.0f64.min(1.0 - (ctx.zoom as f64 - 7.0).ln() / 5.0);

    let context = ctx.context;

    context.push_group(); // top

    if ctx.zoom >= 15 {
        bridge_areas::render(ctx, client, true); // mask
    }

    // CC = (mask, (contours-$cc, final-$cc):src-in, mask-$cut1:dst-out, mask-$cut2:dst-out, ...):src-over

    // (CC, CC, CC, (mask-$cc, mask-$cc, mask-$cc, (fallback_contours, fallback_final):src-out):src-over)

    let config: Vec<(&str, Vec<&str>)> = vec![
        ("at", vec!["sk", "si", "cz"]),
        ("it", vec!["at", "ch", "si", "fr"]),
        ("ch", vec!["at", "fr"]),
        ("si", vec![]),
        ("cz", vec!["sk", "pl"]),
        ("pl", vec!["sk"]),
        ("sk", vec![]),
        ("fr", vec![]),
    ];

    for (country, ccs) in config {
        context.push_group(); // country-contours-and-shading

        hillshading::render(
            ctx,
            &format!("{}-mask", country),
            1.0,
            hillshading_datasets,
            hillshade_scale,
        );

        context.push_group(); // contours-and-shading

        if contours && ctx.zoom >= 12 {
            context.push_group(); // contours
            contours::render(ctx, client, Some(country));
            context.pop_group_to_source().unwrap(); // contours
            context.paint_with_alpha(0.33).unwrap();
        }

        if shading {
            hillshading::render(
                ctx,
                country,
                fade_alpha,
                hillshading_datasets,
                hillshade_scale,
            );
        }

        context.pop_group_to_source().unwrap(); // contours-and-shading
        context.set_operator(cairo::Operator::In);
        context.paint().unwrap();

        if shading {
            for cc in ccs {
                context.set_operator(cairo::Operator::DestOut);
                hillshading::render(
                    ctx,
                    &format!("{}-mask", cc),
                    1.0,
                    hillshading_datasets,
                    hillshade_scale,
                );
            }
        }

        context.pop_group_to_source().unwrap(); // // country-contours-and-shading
        context.paint().unwrap();
    }

    if FALLBACK {
        context.push_group(); // mask

        for country in ["it", "at", "ch", "si", "pl", "sk", "cz", "fr"] {
            hillshading::render(
                ctx,
                &format!("{}-mask", country),
                1.0,
                hillshading_datasets,
                hillshade_scale,
            );
        }

        context.push_group(); // fallback

        {
            let _span = tracy_client::span!("shading_and_contours::contours");

            if contours && ctx.zoom >= 12 {
                context.push_group(); // contours
                contours::render(ctx, client, None);
                context.pop_group_to_source().unwrap(); // contours
                context.paint_with_alpha(0.33).unwrap();
            }
        }

        if shading {
            hillshading::render(ctx, "_", fade_alpha, hillshading_datasets, hillshade_scale);
        }

        context.pop_group_to_source().unwrap(); // fallback
        context.set_operator(cairo::Operator::Out);
        context.paint().unwrap();

        context.pop_group_to_source().unwrap(); // mask
        context.paint().unwrap();
    }

    context.pop_group_to_source().unwrap(); // top
    context.paint().unwrap();
}
