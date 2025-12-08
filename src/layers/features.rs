use crate::colors::{self, Color};
use crate::draw::create_pango_layout::FontAndLayoutOptions;
use crate::draw::text::{TextOptions, draw_text, draw_text_with_attrs};
use crate::projectable::Projectable;
use crate::{bbox::BBox, collision::Collision, ctx::Ctx};
use core::f64;
use geo::Coord;
use pangocairo::pango::{AttrList, AttrSize, SCALE, Style, Weight};
use postgis::ewkb::Point;
use postgres::Client;
use regex::Regex;
use std::u32;
use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

struct Extra<'a> {
    replacements: Vec<(Regex, &'a str)>,
    icon: Option<&'a str>,
    font_size: f64,
    weight: Weight,
    text_color: Color,
    max_zoom: u32,
}

impl Default for Extra<'_> {
    fn default() -> Self {
        Self {
            replacements: vec![],
            icon: None,
            font_size: 12.0,
            weight: Weight::Normal,
            text_color: colors::BLACK,
            max_zoom: u32::MAX,
        }
    }
}

fn build_replacements(pairs: &[(&str, &'static str)]) -> Vec<(Regex, &'static str)> {
    pairs
        .iter()
        .map(|(pattern, replacement)| (Regex::new(pattern).unwrap(), *replacement))
        .collect()
}

struct Def {
    min_zoom: u32,
    min_text_zoom: u32,
    with_ele: bool,
    natural: bool,
    extra: Extra<'static>,
}

#[rustfmt::skip]
static POIS: LazyLock<HashMap<&'static str, Def>> = LazyLock::new(|| {
    const Y: bool = true;
    const N: bool = false;
    const NN: u32 = u32::MAX;

    let spring_replacements = build_replacements(&[
        (r"\b[Mm]inerálny\b", "min."),
        (r"\b[Pp]rameň\b", "prm."),
        (r"\b[Ss]tud(ničk|ň)a\b", "stud."),
        (r"\b[Vv]yvieračka\b", "vyv."),
    ]);

    let church_replacements = build_replacements(&[
        (r"^[Kk]ostol\b *", ""),
        (r"\b([Ss]vät\w+|Sv\.)", "sv."),
    ]);

    let chapel_replacements = build_replacements(&[
        (r"^[Kk]aplnka\b *", ""),
        (r"\b([Ss]vät\w+|Sv\.)", "sv."),
    ]);

    let school_replacements = build_replacements(&[
        (r"[Zz]ákladná [Šš]kola", "ZŠ"),
        (r"[Zz]ákladná [Uu]melecká [Šš]kola", "ZUŠ"),
        (r"[Ss]tredná [Oo]dborná [Šš]kola", "SOŠ"),
        (r"[Gg]ymnázium ", "gym. "),
        (r" [Gg]ymnázium", " gym."),
        (r"[V]ysoká [Šš]kola", "VŠ"),
    ]);

    let college_replacements = build_replacements(&[
        (r"[Ss]tredná [Oo]dborná [Šš]kola", "SOŠ"),
        (r"[Gg]ymnázium ", "gym. "),
        (r" [Gg]ymnázium", " gym."),
        (r"[V]ysoká [Šš]kola", "VŠ"),
    ]);

    let university_replacements = build_replacements(&[(r"[V]ysoká [Šš]kola", "VŠ")]);

    let entries = vec![
        (12, 12, N, N, "aerodrome", Extra {
            replacements: build_replacements(&[(r"^[Ll]etisko\b *", "")]),
            ..Extra::default()
        }),
        (12, 12, Y, N, "guidepost", Extra { icon: Some("guidepost_x"), weight: Weight::Bold, max_zoom: 12, ..Extra::default() }),
        (13, 13, Y, N, "guidepost", Extra { icon: Some("guidepost_xx"), weight: Weight::Bold, max_zoom: 13, ..Extra::default() }),
        (14, 14, Y, N, "guidepost", Extra { icon: Some("guidepost_xx"), weight: Weight::Bold, ..Extra::default() }),
        (10, 10, Y, Y, "peak1", Extra { icon: Some("peak"), font_size: 13.0, ..Extra::default() }),
        (11, 11, Y, Y, "peak2", Extra { icon: Some("peak"), font_size: 13.0, ..Extra::default() }),
        (12, 12, Y, Y, "peak3", Extra { icon: Some("peak"), font_size: 13.0, ..Extra::default() }),
        (13, 13, Y, Y, "peak", Extra { font_size: 13.0, ..Extra::default() }),
        (14, 14, N, N, "castle", Extra {
            replacements: build_replacements(&[(r"^[Hh]rad\b *", "")]),
            ..Extra::default()
        }),
        (14, 15, Y, Y, "arch", Extra::default()),
        (14, 15, Y, Y, "cave_entrance", Extra {
            replacements: build_replacements(&[
                (r"^[Jj]jaskyňa\b *", ""),
                (r"\b[Jj]jaskyňa$", "j."),
                (r"\b[Pp]riepasť\b", "p."),
            ]),
            ..Extra::default()
        }),
        (14, 15, Y, Y, "spring", Extra { replacements: spring_replacements.clone(), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, Y, "refitted_spring", Extra { replacements: spring_replacements.clone(), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, Y, "drinking_spring", Extra { replacements: spring_replacements.clone(), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, Y, "not_drinking_spring", Extra { replacements: spring_replacements.clone(), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, Y, "refitted_drinking_spring", Extra { replacements: spring_replacements.clone(), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, Y, "refitted_not_drinking_spring", Extra { replacements: spring_replacements.clone(), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, Y, "hot_spring", Extra { replacements: spring_replacements.clone(), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, Y, "waterfall", Extra {
            replacements: build_replacements(&[
                (r"^[Vv]odopád\b *", ""),
                (r"\b[Vv]odopád$", "vdp."),
            ]),
            text_color: colors::WATER_LABEL,
            ..Extra::default()
        }),
        (14, 15, N, N, "drinking_water", Extra { text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, N, N, "water_point", Extra { text_color: colors::WATER_LABEL, icon: Some("drinking_water"), ..Extra::default() }),
        (14, 15, N, N, "water_well", Extra { text_color: colors::WATER_LABEL, ..Extra::default() }),
        (14, 15, Y, N, "monument", Extra::default()),
        (14, 15, Y, Y, "viewpoint", Extra {
            replacements: build_replacements(&[
                (r"^[Vv]yhliadka\b *", ""),
                (r"\b[Vv]yhliadka$", "vyhl."),
            ]),
            ..Extra::default()
        }),
        (14, 15, Y, N, "mine", Extra { icon: Some("mine"), ..Extra::default() }),
        (14, 15, Y, N, "adit", Extra { icon: Some("mine"), ..Extra::default() }),
        (14, 15, Y, N, "mineshaft", Extra { icon: Some("mine"), ..Extra::default() }),
        (14, 15, Y, N, "disused_mine", Extra::default()),
        (14, 15, Y, N, "hotel", Extra {
            replacements: build_replacements(&[(r"^[Hh]otel\b *", "")]),
            ..Extra::default()
        }),
        (14, 15, Y, N, "chalet", Extra {
            replacements: build_replacements(&[
                (r"^[Cc]hata\b *", ""),
                (r"\b[Cc]hata$", "ch."),
            ]),
            ..Extra::default()
        }),
        (14, 15, Y, N, "hostel", Extra::default()),
        (14, 15, Y, N, "motel", Extra {
            replacements: build_replacements(&[(r"^[Mm]otel\b *", "")]),
            ..Extra::default()
        }),
        (14, 15, Y, N, "guest_house", Extra::default()),
        (14, 15, Y, N, "apartment", Extra::default()),
        (14, 15, Y, N, "wilderness_hut", Extra::default()),
        (14, 15, Y, N, "alpine_hut", Extra::default()),
        (14, 15, Y, N, "camp_site", Extra::default()),
        (14, 15, N, N, "attraction", Extra::default()),
        (14, 15, N, N, "hospital", Extra {
            replacements: build_replacements(&[(r"^[Nn]emocnica\b", "Nem.")]),
            ..Extra::default()
        }),
        (14, NN, N, N, "townhall", Extra {
            replacements: chapel_replacements.clone(),
            ..Extra::default()
        }),
        (14, 15, N, N, "chapel", Extra::default()),
        (14, 15, N, N, "church", Extra {
            replacements: church_replacements.clone(),
            ..Extra::default()
        }),
        (14, 15, N, N, "cathedral", Extra {
            replacements: church_replacements.clone(),
            icon: Some("church"),
            ..Extra::default()
        }),
        (14, 15, N, N, "synagogue", Extra::default()),
        (14, 15, N, N, "mosque", Extra::default()),
        (14, 15, Y, N, "tower_observation", Extra::default()),
        (14, 15, Y, N, "archaeological_site", Extra::default()),
        (14, 15, N, N, "station", Extra::default()),
        (14, 15, N, N, "halt", Extra { icon: Some("station"), ..Extra::default() }),
        (14, 15, N, N, "bus_station", Extra::default()),
        (14, 15, N, N, "water_park", Extra::default()),
        (14, 15, N, N, "museum", Extra::default()),
        (14, 15, N, N, "manor", Extra::default()),
        (14, 15, N, N, "free_flying", Extra::default()),
        (14, 15, N, N, "forester's_lodge", Extra::default()),
        (14, 15, N, N, "horse_riding", Extra::default()),
        (14, 15, N, N, "golf_course", Extra::default()),
        // TODO (14, 14, N, N, "recycling", Extra { text_color: colors::AREA_LABEL, ..Extra::default() }), // { icon: null } // has no icon yet - render as area name
        (15, NN, Y, N, "guidepost_noname", Extra { icon: Some("guidepost_x"), ..Extra::default() }),
        (15, 15, Y, Y, "saddle", Extra { font_size: 13.0, ..Extra::default() }),
        (15, 16, N, N, "ruins", Extra::default()),
        (15, 16, N, N, "chimney", Extra::default()),
        (15, 16, N, N, "fire_station", Extra {
            replacements: build_replacements(&[(r"^([Hh]asičská zbrojnica|[Pp]ožiarná stanica)\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "community_centre", Extra {
            replacements: build_replacements(&[(r"\b[Cc]entrum voľného času\b", "CVČ")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "police", Extra {
            replacements: build_replacements(&[(r"^[Pp]olícia\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "office", Extra::default()),           // information=office
        (15, 16, N, N, "hunting_stand", Extra::default()),
        (15, 16, Y, N, "shelter", Extra::default()),
        // (15, 16, Y, N, 'shopping_cart', Extra::default()),
        (15, 16, Y, N, "lean_to", Extra::default()),
        (15, 16, Y, N, "public_transport", Extra::default()),
        (15, 16, Y, N, "picnic_shelter", Extra::default()),
        (15, 16, Y, N, "basic_hut", Extra::default()),
        (15, 16, Y, N, "weather_shelter", Extra::default()),
        (15, 16, N, N, "pharmacy", Extra {
            replacements: build_replacements(&[(r"^[Ll]ekáreň\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "cinema", Extra {
            replacements: build_replacements(&[(r"^[Kk]ino\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "theatre", Extra {
            replacements: build_replacements(&[(r"^[Dd]ivadlo\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "memorial", Extra {
            replacements: build_replacements(&[(r"^[Pp]amätník\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "pub", Extra::default()),
        (15, 16, N, N, "cafe", Extra {
            replacements: build_replacements(&[(r"^[Kk]aviareň\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "bar", Extra::default()),
        (15, 16, N, N, "restaurant", Extra {
            replacements: build_replacements(&[(r"^[Rr]eštaurácia\b *", "")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "convenience", Extra::default()),
        (15, 16, N, N, "supermarket", Extra::default()),
        (15, 16, N, N, "fast_food", Extra::default()),
        (15, 16, N, N, "confectionery", Extra::default()),
        (15, 16, N, N, "pastry", Extra { icon: Some("confectionery"), ..Extra::default() }),
        (15, 16, N, N, "fuel", Extra::default()),
        (15, 16, N, N, "post_office", Extra::default()),
        (15, 16, N, N, "bunker", Extra::default()),
        (15, NN, N, N, "mast_other", Extra::default()),
        (15, NN, N, N, "tower_other", Extra::default()),
        (15, NN, N, N, "tower_communication", Extra::default()),
        (
            15,
            NN,
            N,
            N,
            "mast_communication",
            Extra { icon: Some("tower_communication"), ..Extra::default() },
        ),
        (15, 16, N, N, "tower_bell_tower", Extra::default()),
        (15, 16, N, N, "water_tower", Extra::default()),
        (15, 16, N, N, "bus_stop", Extra::default()),
        (15, 16, N, N, "sauna", Extra::default()),
        (15, 16, N, N, "taxi", Extra::default()),
        (15, 16, N, N, "bicycle", Extra::default()),
        (15, 15, N, Y, "tree_protected", Extra { text_color: colors::TREE, ..Extra::default() }),
        (15, 15, N, Y, "tree", Extra::default()),
        (15, 16, N, N, "bird_hide", Extra::default()),
        (15, 16, N, N, "dam", Extra { text_color: colors::WATER_LABEL, ..Extra::default() }),
        (15, 16, N, N, "school", Extra { replacements: school_replacements.clone(), ..Extra::default() }),
        (15, 16, N, N, "college", Extra { replacements: college_replacements.clone(), ..Extra::default() }),
        (15, 16, N, N, "university", Extra { replacements: university_replacements.clone(), ..Extra::default() }),
        (15, 16, N, N, "kindergarten", Extra {
            replacements: build_replacements(&[(r"[Mm]atersk(á|ou) [Šš]k[oô]lk?(a|ou)", "MŠ")]),
            ..Extra::default()
        }),
        (15, 16, N, N, "climbing", Extra::default()),
        (15, 16, N, N, "shooting", Extra::default()),
        (16, 17, N, Y, "rock", Extra::default()),
        (16, 17, N, Y, "stone", Extra::default()),
        (16, 17, N, Y, "sinkhole", Extra::default()),
        (16, 17, N, N, "building", Extra::default()),
        (16, 17, N, N, "weir", Extra { text_color: colors::WATER_LABEL, ..Extra::default() }),
        (16, 17, N, N, "miniature_golf", Extra::default()),
        (16, 17, N, N, "soccer", Extra::default()),
        (16, 17, N, N, "tennis", Extra::default()),
        (16, 17, N, N, "basketball", Extra::default()),
        (16, NN, Y, N, "guidepost_noname", Extra { icon: Some("guidepost_x"), ..Extra::default() }),
        (16, NN, Y, N, "route_marker", Extra { icon: Some("guidepost_x"), ..Extra::default() }),
        (16, NN, N, N, "picnic_table", Extra::default()),
        (16, NN, N, N, "outdoor_seating", Extra::default()),
        (16, 17, N, N, "picnic_site", Extra::default()),
        (16, 16, N, N, "board", Extra::default()),
        (16, 17, N, N, "map", Extra::default()),
        (16, 17, N, N, "artwork", Extra::default()),
        (16, 17, N, N, "fountain", Extra { text_color: colors::WATER_LABEL, ..Extra::default() }),
        (16, NN, N, N, "watering_place", Extra { text_color: colors::WATER_LABEL, ..Extra::default() }),
        (16, NN, N, N, "feeding_place", Extra { icon: Some("manger"), ..Extra::default() }),
        (16, NN, N, N, "game_feeding", Extra { icon: Some("manger"), ..Extra::default() }),
        (16, 17, N, N, "playground", Extra {
            replacements: build_replacements(&[(r"^[Dd]etské ihrisko\b", "")]),
            ..Extra::default()
        }),
        (16, 17, N, N, "water_works", Extra { text_color: colors::WATER_LABEL, ..Extra::default() }),
        (16, 17, N, N, "reservoir_covered", Extra { icon: Some("water_works"), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (16, 17, N, N, "pumping_station", Extra { icon: Some("water_works"), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (16, 17, N, N, "wastewater_plant", Extra { icon: Some("water_works"), text_color: colors::WATER_LABEL, ..Extra::default() }),
        (16, 17, N, N, "cross", Extra::default()),
        (17, 18, N, N, "boundary_stone", Extra::default()),
        (17, 18, N, N, "marker", Extra { icon: Some("boundary_stone"), ..Extra::default() }),
        (17, 18, N, N, "wayside_shrine", Extra::default()),
        (17, 18, N, N, "cross", Extra::default()), // NOTE cross is also on lower zoom
        (17, 18, N, N, "wayside_cross", Extra { icon: Some("cross"), ..Extra::default() }), // NOTE cross is also on lower zoom
        (17, 18, N, N, "tree_shrine", Extra { icon: Some("cross"), ..Extra::default() }), // NOTE cross is also on lower zoom
        (17, NN, N, N, "firepit", Extra::default()),
        (17, NN, N, N, "toilets", Extra::default()),
        (17, NN, N, N, "bench", Extra::default()),
        (17, 18, N, N, "beehive", Extra::default()),
        (17, 18, N, N, "apiary", Extra { icon: Some("beehive"), ..Extra::default() }),
        (17, NN, N, N, "lift_gate", Extra::default()),
        (17, NN, N, N, "swing_gate", Extra { icon: Some("lift_gate"), ..Extra::default() }),
        (17, NN, N, N, "ford", Extra::default()),
        (17, 19, N, N, "parking", Extra { font_size: 10.0, text_color: colors::AREA_LABEL, ..Extra::default() }), // { font: { haloOpacity: 0.5 } },
        (18, 19, N, N, "building_ruins", Extra { icon: Some("ruins"), ..Extra::default() }),
        (18, 19, N, N, "post_box", Extra::default()),
        (18, 19, N, N, "telephone", Extra::default()),
        (18, NN, N, N, "gate", Extra::default()),
        (18, NN, N, N, "waste_disposal", Extra::default()),
        (19, NN, N, N, "waste_basket", Extra::default()),
        ];

    entries
        .into_iter()
        .map(|(min_zoom, min_text_zoom, with_ele, natural, name, extra)| {
            (name, Def {
                min_zoom,
                min_text_zoom,
                with_ele,
                natural,
                extra,
            })
        })
        .collect()
});

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let _span = tracy_client::span!("features::render");

    let Ctx {
        context,
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let mut sql = r#"SELECT * FROM (
        SELECT
            osm_id,
            geometry,
            name AS n,
            tags->'ele' AS ele,
            null AS access,
            isolation,
            CASE WHEN isolation > 4500 THEN 'peak1'
                WHEN isolation BETWEEN 3000 AND 4500 THEN 'peak2'
                WHEN isolation BETWEEN 1500 AND 3000 THEN 'peak3'
                ELSE 'peak'
            END AS type
        FROM osm_features
        NATURAL LEFT JOIN isolations
        WHERE
            type = 'peak' AND
            name <> ''"#
        .to_string();

    if zoom >= 13 {
        sql.push_str(
            r#"
                UNION ALL

                SELECT
                    osm_id,
                    geometry,
                    name AS n,
                    ele,
                    null AS access,
                    null AS isolation,
                    CASE
                        WHEN type <> 'guidepost' THEN type
                        WHEN name = '' THEN 'guidepost_noname'
                        ELSE 'guidepost'
                    END AS type
                FROM osm_infopoints"#,
        );
    }

    if zoom >= 12 && zoom < 14 {
        sql.push_str(
            r#"
                UNION ALL

                SELECT
                    osm_id,
                    geometry,
                    name AS n,
                    tags->'ele',
                    null AS access,
                    null AS isolation,
                    type
                FROM osm_features
                WHERE
                    type = 'aerodrome' AND
                    tags ? 'icao'

                UNION ALL
                    SELECT
                        osm_id,
                        ST_PointOnSurface(geometry) AS geometry,
                        name AS n,
                        tags->'ele',
                        null AS access,
                        null AS isolation,
                        type
                    FROM osm_feature_polys
                    WHERE
                        type = 'aerodrome' AND
                        tags ? 'icao'
          "#,
        );
    }

    if zoom >= 14 {
        sql.push_str(r#"
            UNION ALL

            SELECT
                osm_id,
                geometry,
                name AS n,
                null AS ele,
                tags->'access' AS access,
                null AS isolation,
                type
            FROM osm_sports
            WHERE type IN ('free_flying', 'soccer', 'tennis', 'basketball', 'climbing', 'shooting')

            UNION ALL

            SELECT
                osm_id,
                geometry,
                name AS n,
                tags->'ele' AS ele,
                tags->'access' AS access,
                null AS isolation,
                CASE
                    WHEN type = 'tree' AND tags->'protected' <> 'no' THEN 'tree_protected'
                    WHEN type = 'communications_tower' THEN 'tower_communication'
                    WHEN type = 'shelter' AND tags->'shelter_type' IN ('shopping_cart', 'lean_to', 'public_transport', 'picnic_shelter', 'basic_hut', 'weather_shelter') THEN tags->'shelter_type'
                    WHEN type IN ('mine', 'adit', 'mineshaft') AND tags->'disused' <> 'no' THEN 'disused_mine'
                    ELSE type
                END AS type
            FROM
                osm_features
            WHERE
                type <> 'peak'
                AND (type <> 'tree' OR tags->'protected' NOT IN ('', 'no') OR tags->'denotation' = 'natural_monument')
                AND (type <> 'saddle' OR name <> '')

            UNION ALL

            SELECT
                osm_id,
                ST_PointOnSurface(geometry) AS geometry,
                name AS n,
                tags->'ele' AS ele,
                tags->'access' AS access,
                null AS isolation,
                CASE
                    WHEN type = 'communications_tower' THEN 'tower_communication'
                    WHEN type = 'shelter' AND tags->'shelter_type' IN ('shopping_cart', 'lean_to', 'public_transport', 'picnic_shelter', 'basic_hut', 'weather_shelter') THEN tags->'shelter_type'
                    WHEN type IN ('mine', 'adit', 'mineshaft') AND tags->'disused' NOT IN ('', 'no') THEN 'disused_mine'
                    ELSE type
                END AS type
            FROM osm_feature_polys

            UNION ALL

            SELECT
                osm_id,
                geometry,
                name AS n,
                ele,
                null AS access,
                null AS isolation,
                CASE WHEN type = 'hot_spring' THEN 'hot_spring' ELSE
                    CASE WHEN type = 'spring_box' OR refitted = 'yes' THEN 'refitted_' ELSE '' END ||
                    CASE WHEN drinking_water = 'yes' OR drinking_water = 'treated' THEN 'drinking_' WHEN drinking_water = 'no' THEN 'not_drinking_' ELSE '' END || 'spring'
                END AS type
            FROM osm_springs

            UNION ALL

            SELECT
                osm_id,
                ST_PointOnSurface(geometry) AS geometry,
                name AS n,
                null AS ele,
                null AS access,
                null AS isolation,
                building AS type
            FROM osm_place_of_worships
            WHERE building IN ('chapel', 'church', 'temple', 'mosque', 'cathedral', 'synagogue')

            UNION ALL

            SELECT
                osm_id,
                geometry,
                name AS n,
                ele,
                null AS access,
                null AS isolation,
                CONCAT(
                    "class",
                    '_',
                    CASE type
                        WHEN 'communication' THEN 'communication'
                        WHEN 'observation' THEN 'observation'
                        WHEN 'bell_tower' THEN 'bell_tower'
                        ELSE 'other'
                    END
                ) AS type
            FROM osm_towers
        "#);
    }

    if zoom >= 15 {
        sql.push_str(
                r#"
                    UNION ALL

                    SELECT
                        osm_id,
                        geometry,
                        name AS n,
                        null AS ele,
                        null AS access,
                        null AS isolation,
                        'ruins' AS type
                    FROM osm_ruins

                    UNION ALL

                    SELECT
                        osm_id,
                        geometry,
                        name AS n,
                        null AS ele,
                        null AS access,
                        null AS isolation,
                        type
                    FROM osm_shops
                    WHERE type IN ('convenience', 'fuel', 'confectionery', 'pastry', 'bicycle', 'supermarket')

                    UNION ALL

                    SELECT
                        osm_id,
                        geometry,
                        name AS n,
                        null AS ele,
                        tags->'access' AS access,
                        null AS isolation,
                        CASE type WHEN 'ruins' THEN 'building_ruins' ELSE 'building' END AS type
                    FROM osm_building_points
                    WHERE type <> 'no'

                    UNION ALL

                    SELECT
                        osm_id,
                        ST_LineInterpolatePoint(geometry, 0.5) AS geometry,
                        name AS n,
                        null AS ele,
                        null AS access,
                        null AS isolation,
                        type
                    FROM osm_feature_lines
                    WHERE type IN ('dam', 'weir')
                "#,
            );
    }

    if zoom >= 17 {
        sql.push_str(
            r#"
                    UNION ALL

                    SELECT
                        osm_id,
                        geometry,
                        name AS n,
                        null AS ele,
                        null AS access,
                        null AS isolation,
                        type
                    FROM osm_barrierpoints
                    WHERE type IN ('lift_gate', 'swing_gate', 'gate')

                    UNION ALL

                    SELECT
                        osm_id,
                        geometry,
                        '' AS n,
                        null AS ele,
                        null AS access,
                        null AS isolation,
                        'ford' AS type
                    FROM osm_fords

                    UNION ALL

                    SELECT
                        osm_id,
                        geometry,
                        name AS n,
                        null AS ele,
                        null AS access,
                        null AS isolation,
                        'building_ruins' AS type
                    FROM osm_buildings
                    WHERE type = 'ruins'"#,
        );
    }

    sql.push_str(
        r#"
            ) AS abc
            LEFT JOIN z_order_poi USING (type)
            WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
            ORDER BY
                z_order,
                isolation DESC NULLS LAST,
                ele DESC NULLS LAST,
                osm_id
        "#,
    );

    let mut svg_cache = ctx.svg_cache.borrow_mut();

    let buffer = ctx.meters_per_pixel() * 1024.0;

    let rows = {
        let _span = tracy_client::span!("features::query");
        client
            .query(&sql, &[min_x, min_y, max_x, max_y, &buffer])
            .expect("db data")
    };

    let mut to_label = Vec::<(Coord, f64, String, Option<String>, usize, &Def)>::new();

    {
        let _paint_span = tracy_client::span!("features::paint_svgs");

        for row in rows {
            let typ: &str = row.get("type");

            let def = POIS.get(typ);

            let Some(def) = def else {
                continue;
            };

            if def.min_zoom > zoom || def.extra.max_zoom < zoom {
                continue;
            }

            let geom: Point = row.get("geometry");

            let point = geom.project(ctx);

            let surface = svg_cache.get(&format!(
                "images/{}.svg",
                def.extra.icon.unwrap_or_else(|| typ)
            ));

            let rect = surface.extents().unwrap();

            let x = (point.x - rect.width() / 2.0).round();

            let y = (point.y - rect.height() / 2.0).round();

            'outer: for (j, r) in vec![5.0, 10.0].into_iter().enumerate() {
                for (i, a) in vec![
                    0.0,
                    0.0,
                    f64::consts::TAU / 2.0,
                    f64::consts::TAU / 4.0,
                    3.0 * f64::consts::TAU / 4.0,
                    f64::consts::TAU / 8.0,
                    7.0 * f64::consts::TAU / 8.0,
                    3.0 * f64::consts::TAU / 8.0,
                    5.0 * f64::consts::TAU / 8.0,
                ]
                .into_iter()
                .enumerate()
                {
                    if i == 0 && j > 0 {
                        continue;
                    }

                    let dx = if i == 0 { 0.0 } else { r * f64::sin(a) };
                    let dy = if i == 0 { 0.0 } else { r * f64::cos(a) };

                    let bbox = BBox {
                        min_x: x + dx,
                        min_y: y + dy,
                        max_x: x + dx + rect.width(),
                        max_y: y + dy + rect.height(),
                    };

                    if collision.collides(&bbox) {
                        continue;
                    }

                    let bbox_idx = collision.add(bbox);

                    if def.min_text_zoom <= zoom {
                        let name: &str = row.get("n");

                        if !name.is_empty() {
                            let mut name: Cow<'_, str> = Cow::Borrowed(name);

                            for (regex, replacement) in &def.extra.replacements {
                                if let Cow::Owned(updated) =
                                    regex.replace(name.as_ref(), *replacement)
                                {
                                    name = Cow::Owned(updated);
                                }
                            }

                            to_label.push((
                                Coord {
                                    x: point.x + dx,
                                    y: point.y + dy,
                                },
                                rect.height() / 2.0,
                                name.into_owned(),
                                row.get("ele"),
                                bbox_idx,
                                def,
                            ));
                        }
                    }

                    let _span = tracy_client::span!("features::paint_svg");

                    context.set_source_surface(surface, x + dx, y + dy).unwrap();

                    context.paint().unwrap();

                    break 'outer;
                }
            }
        }
    }

    {
        let _span = tracy_client::span!("features::labels");

        for (point, d, name, ele, bbox_idx, def) in to_label.into_iter() {
            let text_options = TextOptions {
                flo: FontAndLayoutOptions {
                    style: if def.natural {
                        Style::Italic
                    } else {
                        Style::Normal
                    },
                    size: def.extra.font_size,
                    weight: def.extra.weight,
                    ..Default::default()
                },
                color: def.extra.text_color,
                valign_by_placement: true,
                placements: &[-d - 3.0, d - 3.0],
                omit_bbox: Some(bbox_idx),
                ..Default::default()
            };

            let drawn = if def.with_ele
                && let Some(ele) = ele
            {
                let attr_list = AttrList::new();

                let mut scale_attr = AttrSize::new(8 * SCALE);
                scale_attr.set_start_index(name.len() as u32 + 1);

                attr_list.insert(scale_attr);

                draw_text_with_attrs(
                    context,
                    collision,
                    point,
                    &format!("{}\n{}", name, ele).trim(),
                    Some(attr_list),
                    &text_options,
                )
            } else {
                draw_text(context, collision, point, &name, &text_options)
            };

            if !drawn {
                continue;
            }
        }
    }
}
