use crate::{
    ctx::Ctx,
    layers::{bridge_areas, contours, hillshading},
};
use gdal::Dataset;
use postgres::Client;
use std::{cell::RefCell, collections::HashMap};

const FALLBACK: bool = true;
const CONTOURS: bool = true;

thread_local! {
    pub static SHADING_THREAD_LOCAL: RefCell<HashMap<String, Dataset>> = {
        let datasets = [
            (String::from("sk"), "/home/martin/14TB/hillshading/sk/final.tif"),
            (String::from("sk-mask"), "/home/martin/14TB/hillshading/sk/mask.tif"),
            (String::from("cz"), "/home/martin/14TB/hillshading/cz/final.tif"),
            (String::from("cz-mask"), "/home/martin/14TB/hillshading/cz/mask.tif"),
            (String::from("at"), "/home/martin/14TB/hillshading/at/final.tif"),
            (String::from("at-mask"), "/home/martin/14TB/hillshading/at/mask.tif"),
            (String::from("pl"), "/home/martin/14TB/hillshading/pl/final.tif"),
            (String::from("pl-mask"), "/home/martin/14TB/hillshading/pl/mask.tif"),
            (String::from("it"), "/home/martin/14TB/hillshading/it/final.tif"),
            (String::from("it-mask"), "/home/martin/14TB/hillshading/it/mask.tif"),
            (String::from("ch"), "/home/martin/14TB/hillshading/ch/final.tif"),
            (String::from("ch-mask"), "/home/martin/14TB/hillshading/ch/mask.tif"),
            (String::from("si"), "/home/martin/14TB/hillshading/si/final.tif"),
            (String::from("si-mask"), "/home/martin/14TB/hillshading/si/mask.tif"),
            (String::from("fr"), "/home/martin/14TB/hillshading/fr/final.tif"),
            (String::from("fr-mask"), "/home/martin/14TB/hillshading/fr/mask.tif"),
            (String::from("_"), "/home/martin/14TB/hillshading/final.tiff"),
        ];

        let mut hillshading_datasets = HashMap::new();

        for (name, path) in datasets {
            match Dataset::open(path) {
                Ok(dataset) => {
                    hillshading_datasets.insert(name.clone(), dataset);
                }
                Err(err) => {
                    eprintln!("Error opening hillshading geotiff {}: {}", path, err);
                }
            }
        }

        RefCell::new(hillshading_datasets)
    };
}

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx { context, zoom, .. } = ctx;

    let fade_alpha = 1.0f64.min(1.0 - (*zoom as f64 - 7.0).ln() / 5.0);

    context.push_group(); // top

    if *zoom >= 15 {
        bridge_areas::render(ctx, client, true); // mask
    }

    // CC = (mask, (contours-$cc, final-$cc):src-in, mask-$cut1:dst-out, mask-$cut2:dst-out, ...):src-over

    // (CC, CC, CC, (mask-$cc, mask-$cc, mask-$cc, (fallback_contours, fallback_final):src-out):src-over)

    let config: Vec<(&str, Vec<&str>)> = vec![
        // ("at", vec!["sk", "si", "cz"]),
        // ("it", vec!["at", "ch", "si", "fr"]),
        // ("ch", vec!["at", "fr"]),
        // ("si", vec![]),
        // ("cz", vec!["sk", "pl"]),
        // ("pl", vec!["sk"]),
        ("sk", vec![]),
        // ("fr", vec![]),
    ];

    for (country, ccs) in config {
        context.push_group(); // country-contours-and-shading

        hillshading::render(ctx, &format!("{}-mask", country), 1.0);

        context.push_group(); // contours-and-shading

        if CONTOURS && *zoom >= 12 {
            context.push_group(); // contours
            contours::render(ctx, client, country);
            context.pop_group_to_source().unwrap(); // contours
            context.paint().unwrap();
        }

        hillshading::render(ctx, country, fade_alpha);

        context.pop_group_to_source().unwrap(); // contours-and-shading
        context.set_operator(cairo::Operator::In);
        context.paint().unwrap();

        for cc in ccs {
            context.set_operator(cairo::Operator::DestOut);
            hillshading::render(ctx, &format!("{}-mask", cc), 1.0);
        }

        context.pop_group_to_source().unwrap(); // // country-contours-and-shading
        context.paint().unwrap();
    }

    if FALLBACK {
        context.push_group(); // mask

        for country in vec!["it", "at", "ch", "si", "pl", "sk", "cz", "fr"] {
            hillshading::render(ctx, &format!("{}-mask", country), 1.0);
        }

        context.push_group(); // fallback

        if CONTOURS && *zoom >= 12 {
            context.push_group(); // contours
            contours::render(ctx, client, "contour_split");
            context.pop_group_to_source().unwrap(); // contours
            context.paint().unwrap();
        }

        hillshading::render(ctx, "_", fade_alpha);

        context.pop_group_to_source().unwrap(); // fallback
        context.set_operator(cairo::Operator::Out);
        context.paint().unwrap();

        context.pop_group_to_source().unwrap(); // mask
        context.paint().unwrap();
    }

    context.pop_group_to_source().unwrap(); // top
    context.paint().unwrap();
}
