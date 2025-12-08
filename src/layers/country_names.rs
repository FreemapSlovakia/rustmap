use geojson::{FeatureCollection, GeoJson, Geometry, Value};
use std::fs::File;

use crate::ctx::Ctx;

pub fn render(ctx: &Ctx) {
    let file = File::open("/home/martin/fm/freemap-mapnik/country-names.geojson").unwrap();

    let geojson = GeoJson::from_reader(file).unwrap();

    let feature_collection = FeatureCollection::try_from(geojson).unwrap();

    for feature in feature_collection {
        if let Some(Geometry {
            value: Value::LineString(line_string),
            ..
        }) = feature.geometry
        {
            // line_string.
        }
    }
}
