use gdal::Dataset;
use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
};

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

const MAX_UNUSED_USES: u64 = 100;

struct CachedDataset {
    dataset: Dataset,
    last_use: u64,
}

pub struct HillshadingDatasets {
    base: PathBuf,
    datasets: HashMap<String, CachedDataset>,
    use_counter: u64,
}

impl HillshadingDatasets {
    pub fn new(base: impl AsRef<Path>) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            datasets: HashMap::new(),
            use_counter: 0,
        }
    }

    pub fn evict_unused(&mut self) {
        let threshold = self.use_counter.saturating_sub(MAX_UNUSED_USES);
        self.datasets
            .retain(|_, cached| cached.last_use >= threshold);
    }

    pub fn get(&mut self, name: &str) -> Option<&Dataset> {
        match self.datasets.entry(name.to_string()) {
            Entry::Occupied(occ) => Some(&occ.into_mut().dataset),
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
                            last_use: self.use_counter,
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

    pub fn record_use(&mut self, name: &str) {
        self.use_counter = self.use_counter.saturating_add(1);

        if let Some(entry) = self.datasets.get_mut(name) {
            entry.last_use = self.use_counter;
        }

        self.evict_unused();
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
