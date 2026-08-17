use crate::render::{
    Feature, PlaceTypeOverride,
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        font_options::FontAndLayoutOptions,
        text::{TextOptions, draw_text},
    },
    layer_render_error::LayerRenderResult,
    projectable::TileProjectable,
};
use cairo::Context;
use cosmic_text::{Style, Weight};

/// A named place label style. The set is deliberately small — every place label on the map
/// is drawn in one of these — so that a country override picks an existing size instead of
/// inventing one.
#[derive(Debug)]
pub struct LabelStyle {
    pub name: &'static str,
    size: f64,
    uppercase: bool,
    halo_width: f64,
}

#[cfg_attr(any(), rustfmt::skip)]
pub const LABEL_STYLES: &[LabelStyle] = &[
    LabelStyle { name: "xxl", size: 1.20, uppercase: true,  halo_width: 2.0 },
    LabelStyle { name: "xl",  size: 0.80, uppercase: true,  halo_width: 2.0 },
    LabelStyle { name: "l",   size: 0.55, uppercase: true,  halo_width: 1.5 },
    LabelStyle { name: "m",   size: 0.50, uppercase: false, halo_width: 1.5 },
    LabelStyle { name: "s",   size: 0.45, uppercase: false, halo_width: 1.5 },
    LabelStyle { name: "xs",  size: 0.40, uppercase: false, halo_width: 1.5 },
    LabelStyle { name: "xxs", size: 0.35, uppercase: false, halo_width: 1.5 },
];

/// Highest zoom the layer is drawn at, so a rule cannot postpone a label out of the map.
pub const MAX_PLACE_ZOOM: u8 = 17;

/// The default rule of every labelled place type: from which zoom, in which style. Country
/// overrides (see `PlaceTypeOverrides`) replace the pair for one type; the SQL type filter
/// is derived from the zooms here, so a type missing from this table is never even fetched.
/// `island`/`islet` are absent on purpose — their visibility follows the mapped area.
const PLACE_DEFAULTS: &[(&str, u8, &str)] = &[
    ("city", 8, "xxl"),
    ("town", 9, "xl"),
    ("village", 11, "l"),
    ("hamlet", 12, "m"),
    ("allotments", 12, "m"),
    ("suburb", 12, "m"),
    ("isolated_dwelling", 14, "s"),
    ("quarter", 14, "s"),
    ("neighbourhood", 15, "xs"),
    ("farm", 16, "xxs"),
    ("borough", 16, "xxs"),
    ("square", 16, "xxs"),
];

pub fn label_style(name: &str) -> Option<&'static LabelStyle> {
    LABEL_STYLES.iter().find(|style| style.name == name)
}

/// The zoom and style a place type is drawn with unless a country override replaces them.
pub fn place_type_default(place_type: &str) -> Option<(u8, &'static LabelStyle)> {
    PLACE_DEFAULTS
        .iter()
        .find(|(candidate, _, _)| *candidate == place_type)
        .map(|(_, min_zoom, style)| {
            (
                *min_zoom,
                label_style(style).expect("place default names a known style"),
            )
        })
}

/// Lowest zoom at which a place type gets a label, including the area-gated `island`/`islet`
/// which have no default style. Used to keep overrides from showing a type earlier.
pub fn place_type_min_zoom(place_type: &str) -> Option<u8> {
    if matches!(place_type, "island" | "islet") {
        return Some(8);
    }

    place_type_default(place_type).map(|(min_zoom, _)| min_zoom)
}

/// Fetch exactly the types that can be drawn at this zoom. Overrides may only postpone a
/// type, so this stays a superset of what `render` ends up keeping.
fn types_for_zoom(zoom: u8) -> String {
    let types = PLACE_DEFAULTS
        .iter()
        .filter(|(_, min_zoom, _)| *min_zoom <= zoom)
        .map(|(place_type, _, _)| *place_type)
        .chain(["island", "islet"])
        .map(|place_type| format!("'{place_type}'"))
        .collect::<Vec<_>>()
        .join(", ");

    format!("a.type IN ({types})")
}

pub async fn query(
    ctx: &Ctx,
    client: &tokio_postgres::Client,
) -> Result<Vec<tokio_postgres::Row>, tokio_postgres::Error> {
    let zoom = ctx.zoom;

    if zoom < 8 {
        return Ok(Vec::new());
    }

    let by_zoom = types_for_zoom(zoom);

    // Only look the country up when there is something to apply it to; without overrides
    // the whole join is skipped and `render` never reads the columns.
    let (country_column, country_join) = if ctx.place_type_overrides.is_some() {
        (
            ", COALESCE(c.country, '') AS country, COALESCE(p.population, 0) AS population",
            // Claimed-by-both areas (Crimea) sit in two countries; order so the pick is stable.
            "LEFT JOIN LATERAL (
                SELECT c.country FROM countries c WHERE c.geometry && p.geometry AND ST_Intersects(c.geometry, p.geometry) ORDER BY c.country LIMIT 1
            ) c ON TRUE",
        )
    } else {
        ("", "")
    };

    #[cfg_attr(any(), rustfmt::skip)]
    let sql = format!("
        WITH p AS MATERIALIZED (
            SELECT
                a.name,
                a.type,
                COALESCE(a.area, 0) AS area,
                ST_PointOnSurface(a.geometry) AS geometry,
                a.z_order,
                a.population,
                a.osm_id
            FROM
                osm_places a LEFT JOIN osm_places b ON a.name = b.name AND a.osm_id <> b.osm_id AND ST_Contains(a.geometry, b.geometry)
            WHERE
                    {by_zoom} AND
                    a.name <> '' AND
                    a.geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5) AND
                    b.osm_id IS NULL
        )
        SELECT
            p.name,
            p.type,
            p.area,
            p.geometry{country_column}
        FROM
            p {country_join}
        ORDER BY
            p.z_order DESC,
            p.population DESC NULLS LAST,
            p.osm_id
    ");

    client
        .query(&sql, &ctx.bbox_query_params(Some(1024.0)).as_params())
        .await
}

pub fn render(
    ctx: &Ctx,
    context: &Context,
    rows: Vec<Feature>,
    collision: &mut Option<&mut Collision>,
) -> LayerRenderResult {
    let _span = tracy_client::span!("place_names::render");

    let zoom = ctx.zoom;

    let positions = [
        (0.0, -10.0),
        (0.0, 10.0),
        (-30.0, 0.0),
        (30.0, 0.0),
        (-25.0, -8.0),
        (-25.0, 8.0),
        (25.0, -8.0),
        (25.0, 8.0),
    ];

    let scale = 2.5 * 1.2f64.powf(zoom.min(14) as f64);

    for row in rows {
        let mut color = colors::BLACK;
        let mut letter_spacing = 1.0;

        let place_type = row.get_string("type")?;

        let mut rule = None;

        if let Some(overrides) = ctx.place_type_overrides.as_ref() {
            match overrides.get(
                row.get_string("country")?,
                place_type,
                row.get_i32("population")?,
            ) {
                Some(PlaceTypeOverride::RenderAs { min_zoom, style }) => {
                    rule = Some((min_zoom, style));
                }
                Some(PlaceTypeOverride::Hide) => continue,
                None => {}
            }
        }

        let resolved = rule.or_else(|| place_type_default(place_type));

        let (size, uppercase, halo_width, italic) = if let Some((min_zoom, style)) = resolved {
            if zoom < min_zoom {
                continue;
            }

            (style.size, style.uppercase, style.halo_width, false)
        } else if matches!(place_type, "island" | "islet") {
            // Not on the zoom ladder: these show once the mapped area is big enough to see.
            let mut area = row.get_f32("area")?;

            if area == 0.0 {
                area = 10000.0;
            }

            if area < 4f32.powf(22f32 - zoom as f32) {
                continue;
            }

            color = colors::LOCALITY_LABEL;
            letter_spacing = 0.0;

            (
                0.4 * (1.0 + area.sqrt() / 2000.0).min(2.0) as f64,
                false,
                1.5,
                true,
            )
        } else {
            continue;
        };

        // TODO could be precomputed
        let mut placements = Vec::with_capacity(41);
        placements.push((0.0, 0.0));

        for i in 1..6 {
            for p in &positions {
                placements.push((2.0 * size * p.0 * i as f64, 2.0 * size * p.1 * i as f64));
            }
        }

        draw_text(
            context,
            collision.as_deref_mut(),
            &row.get_point()?.project_to_tile(&ctx.tile_projector),
            row.get_string("name")?,
            &TextOptions {
                flo: FontAndLayoutOptions {
                    size: size * scale,
                    max_width: 8.0 * size * scale,
                    uppercase,
                    narrow: true,
                    weight: Weight::BOLD,
                    letter_spacing,
                    style: if italic { Style::Italic } else { Style::Normal },
                },
                halo_width,
                halo_opacity: 0.9,
                placements: &placements,
                color,
                ..TextOptions::default()
            },
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LABEL_STYLES, PLACE_DEFAULTS, label_style, types_for_zoom};

    #[test]
    fn every_default_names_a_known_style() {
        for (place_type, _, style) in PLACE_DEFAULTS {
            assert!(
                label_style(style).is_some(),
                "{place_type} uses unknown style {style}"
            );
        }

        for style in LABEL_STYLES {
            assert!(label_style(style.name).is_some());
        }
    }

    #[test]
    fn the_type_filter_follows_the_defaults() {
        assert_eq!(types_for_zoom(8), "a.type IN ('city', 'island', 'islet')");
        assert_eq!(
            types_for_zoom(10),
            "a.type IN ('city', 'town', 'island', 'islet')"
        );
        assert_eq!(
            types_for_zoom(12),
            "a.type IN ('city', 'town', 'village', 'hamlet', 'allotments', 'suburb', 'island', 'islet')"
        );
        // Types the layer never draws (locality, municipality, …) are not fetched at all.
        assert!(!types_for_zoom(17).contains("locality"));
        assert!(types_for_zoom(17).contains("'farm'"));
    }
}
