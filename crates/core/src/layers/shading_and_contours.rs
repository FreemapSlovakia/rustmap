use crate::{
    ctx::Ctx,
    layers::{bridge_areas, contours, hillshading},
};
use gdal::Dataset;
use postgres::Client;
use std::{collections::HashMap, path::Path};

const FALLBACK: bool = true;

pub fn load_hillshading_datasets(base: impl AsRef<Path>) -> HashMap<String, Dataset> {
    let base = base.as_ref();
    let datasets = [
        (String::from("sk"), "sk/final.tif"),
        (String::from("sk-mask"), "sk/mask.tif"),
        (String::from("cz"), "cz/final.tif"),
        (String::from("cz-mask"), "cz/mask.tif"),
        (String::from("at"), "at/final.tif"),
        (String::from("at-mask"), "at/mask.tif"),
        (String::from("pl"), "pl/final.tif"),
        (String::from("pl-mask"), "pl/mask.tif"),
        (String::from("it"), "it/final.tif"),
        (String::from("it-mask"), "it/mask.tif"),
        (String::from("ch"), "ch/final.tif"),
        (String::from("ch-mask"), "ch/mask.tif"),
        (String::from("si"), "si/final.tif"),
        (String::from("si-mask"), "si/mask.tif"),
        (String::from("fr"), "fr/final.tif"),
        (String::from("fr-mask"), "fr/mask.tif"),
        (String::from("_"), "final.tiff"),
    ];

    let mut hillshading_datasets = HashMap::new();

    for (name, path) in datasets {
        let full_path = base.join(path);

        match Dataset::open(&full_path) {
            Ok(dataset) => {
                hillshading_datasets.insert(name.clone(), dataset);
            }
            Err(err) => {
                eprintln!(
                    "Error opening hillshading geotiff {}: {}",
                    full_path.display(),
                    err
                );
            }
        }
    }

    hillshading_datasets
}

pub fn render(
    ctx: &Ctx,
    client: &mut Client,
    hillshading_datasets: &mut HashMap<String, Dataset>,
    shading: bool,
    contours: bool,
    hillshade_scale: f64,
) {
    let _span = tracy_client::span!("shading_and_contours::render");

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
