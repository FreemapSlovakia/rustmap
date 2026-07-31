use crate::render::{FeatureLineMaskCountries, HillshadingHierarchy};
use gdal::Dataset;
use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const EVICT_AFTER: Duration = Duration::from_secs(10);

struct CachedDataset {
    dataset: Dataset,
    last_used_at: Instant,
}

pub struct HillshadingDatasets {
    base: PathBuf,
    /// Dataset names that may be opened. Any request for a name outside this set is
    /// treated as "no dataset" so callers can hand in codes (e.g. hardcoded country
    /// lists) that aren't part of the configured hillshading hierarchy.
    allowed: HashSet<String>,
    datasets: HashMap<String, CachedDataset>,
}

impl HillshadingDatasets {
    pub fn new(base: impl AsRef<Path>, allowed: HashSet<String>) -> Self {
        Self {
            base: base.as_ref().to_path_buf(),
            allowed,
            datasets: HashMap::new(),
        }
    }

    pub fn evict_unused(&mut self) {
        let now = Instant::now();

        self.datasets
            .retain(|_, cached| now.duration_since(cached.last_used_at) <= EVICT_AFTER);
    }

    pub fn get(&mut self, name: &str) -> Option<&Dataset> {
        if !self.allowed.contains(name) {
            return None;
        }

        match self.datasets.entry(name.to_string()) {
            Entry::Occupied(occ) => Some(&occ.into_mut().dataset),
            Entry::Vacant(vac) => {
                let full_path = self.base.join(name).join("final.tif");

                match Dataset::open(&full_path) {
                    Ok(dataset) => {
                        let entry = vac.insert(CachedDataset {
                            dataset,
                            last_used_at: Instant::now(),
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
        if let Some(entry) = self.datasets.get_mut(name) {
            entry.last_used_at = Instant::now();
        }
    }
}

/// Create a lazily-loading dataset cache restricted to the hillshadings referenced by
/// `hierarchy` (plus the `_` global fallback) and by `feature_line_mask_countries`, whose
/// masks may come from countries that are not shaded themselves. Every `better` code is
/// validated to also be a `country` key, so the `country` keys cover the full set of
/// datasets the hierarchy references. Names outside this set are never opened from disk.
pub fn load_hillshading_datasets(
    base: impl AsRef<Path>,
    hierarchy: &HillshadingHierarchy,
    feature_line_mask_countries: Option<&FeatureLineMaskCountries>,
) -> HillshadingDatasets {
    let mut allowed: HashSet<String> = hierarchy
        .entries()
        .iter()
        .map(|entry| entry.country.to_string())
        .collect();

    // Global fallback dataset used where no country mask covers the tile.
    allowed.insert("_".to_string());

    if let Some(feature_line_mask_countries) = feature_line_mask_countries {
        allowed.extend(feature_line_mask_countries.countries().iter().cloned());
    }

    HillshadingDatasets::new(base, allowed)
}
