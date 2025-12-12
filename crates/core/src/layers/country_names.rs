use crate::colors::{self, ContextExt};
use crate::ctx::Ctx;
use crate::draw::create_pango_layout::FontAndLayoutOptions;
use crate::draw::offset_line::offset_line;
use crate::draw::text_on_line::{Align, TextOnLineOptions, text_on_line};
use crate::layers::borders;
use crate::projectable::TileProjectable;
use geo::{BoundingRect, Coord, Geometry, Intersects, LineString};
use geojson::{FeatureCollection, GeoJson};
use postgres::Client;
use proj::{Proj, Transform};
use std::convert::TryFrom;
use std::f64;
use std::fs::File;
use std::sync::LazyLock;

thread_local! {
    static FEATURES: LazyLock<Vec<(LineString, String, String)>> = LazyLock::new(|| {
        let file = File::open("/home/martin/fm/maprender/country-names.geojson").unwrap();

        let geojson = GeoJson::from_reader(file).expect("geojson");

        let proj = Proj::new_known_crs("EPSG:4326", "EPSG:3857", None).expect("transformer init");

        let features: Vec<(LineString, String, String)> = FeatureCollection::try_from(geojson)
            .expect("feature collection")
            .features.iter().filter_map(|feature| {
                let geom: Geometry = feature
                    .geometry
                    .as_ref()
                    .map(|geometry| Geometry::try_from(geometry.clone()).expect("geometry"))
                    .expect("non-null");

                let Geometry::LineString(mut geom) = geom else {
                    return None;
                };

                geom.transform(&proj).expect("transformed");

                let properties = feature.properties.as_ref().expect("properties");

                Some((
                    geom,
                    properties.get("name").expect("name").as_str().expect("string").to_owned(),
                    properties.get("name:en").expect("name:en").as_str().expect("string").to_owned()),
                )
            }).collect();

        features
    });
}

pub fn render(ctx: &Ctx, client: &mut Client) {
    let context = ctx.context;

    let rect = ctx.bbox.project_to_tile(&ctx.tile_projector);

    context.save().unwrap();
    context.rectangle(rect.min().x, rect.min().y, rect.width(), rect.height());
    context.set_source_color_a(colors::WHITE, 0.33);
    context.fill().unwrap();
    context.restore().unwrap();

    borders::render(ctx, client);

    let sr = 1.5f64.powf(ctx.zoom as f64 - 6.0);

    let min = rect.min();
    let max = rect.max();

    let margin = ctx.meters_per_pixel() * sr * 20.0;

    let rect = geo::Rect::new(
        Coord {
            x: min.x - margin,
            y: min.y - margin,
        },
        Coord {
            x: max.x + margin,
            y: max.y + margin,
        },
    );

    let options_upper = TextOnLineOptions {
        flo: FontAndLayoutOptions {
            size: sr * 20.0,
            ..Default::default()
        },
        halo_width: 2.0,
        align: Align::Justify {
            min_spacing: Some(0.0),
        },
        concave_spacing_factor: 0.0,
        ..Default::default()
    };

    let options_lower = TextOnLineOptions {
        flo: FontAndLayoutOptions {
            size: sr * 16.0,
            ..Default::default()
        },
        halo_width: 2.0,
        color: colors::AREA_LABEL,
        align: Align::Justify {
            min_spacing: Some(0.0),
        },
        concave_spacing_factor: 0.0,
        ..Default::default()
    };

    FEATURES.with(|features| {
        for (geom, name, name_en) in features.iter() {
            let geom = geom.project_to_tile(&ctx.tile_projector);

            let bounds = geom.bounding_rect().unwrap();

            if !rect.intersects(&bounds) {
                continue;
            }

            // context.save().unwrap();
            // draw_line(context, &geom);
            // context.set_source_color(colors::BLACK);
            // context.set_line_width(2.0);
            // context.stroke().unwrap();
            // context.restore().unwrap();

            context.push_group();

            for (name, mut offset, options) in vec![
                (name, sr * -14.0, &options_upper),
                (name_en, sr * 8.0, &options_lower),
            ] {
                let mut options = *options;

                for _ in 0..4 {
                    let geom = offset_line(&geom, offset);

                    if text_on_line(context, &geom, name, None, &options) {
                        break;
                    }

                    options.flo.size *= 0.9;
                    options.halo_width *= 0.9;
                    offset *= 0.9;
                }
            }

            context.pop_group_to_source().unwrap();
            context.paint_with_alpha(0.66).unwrap();
        }
    });
}
