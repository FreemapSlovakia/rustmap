use crate::draw::text::{DEFAULT_PLACEMENTS, TextOptions, draw_text};
use crate::{bbox::BBox, collision::Collision, ctx::Ctx, draw::draw::Projectable};
use core::f64;
use postgis::ewkb::Point;
use postgres::Client;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Default)]
struct Extra<'a> {
    replacements: Vec<(Regex, &'a str)>,
    icon: Option<&'a str>,
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
    extra: Option<Extra<'static>>,
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
        (12, 12, N, N, "aerodrome", Some(Extra {
            replacements: build_replacements(&[(r"^[Ll]etisko\b *", "")]),
            ..Extra::default()
        })),
        (12, 12, Y, N, "guidepost", Some(Extra { icon: Some("guidepost_x"), ..Extra::default() })), // { font: { fontsetName: "bold", dy: -8 }, maxZoom: 12 }
        (13, 13, Y, N, "guidepost", Some(Extra { icon: Some("guidepost_xx"), ..Extra::default() })), // { font: { fontsetName: "bold" }, maxZoom: 13 }
        (14, 14, Y, N, "guidepost", Some(Extra { icon: Some("guidepost_xx"), ..Extra::default() })), // { font: { fontsetName: "bold" } }
        (10, 10, Y, Y, "peak1", Some(Extra { icon: Some("peak"), ..Extra::default() })),             // { font: { size: 13, dy: -8 } }
        (11, 11, Y, Y, "peak2", Some(Extra { icon: Some("peak"), ..Extra::default() })),             // { font: { size: 13, dy: -8 } }
        (12, 12, Y, Y, "peak3", Some(Extra { icon: Some("peak"), ..Extra::default() })),             // { font: { size: 13, dy: -8 } }
        (13, 13, Y, Y, "peak", None),                      // { font: { size: 13, dy: -8 } }
        (14, 14, N, N, "castle", Some(Extra {
            replacements: build_replacements(&[(r"^[Hh]rad\b *", "")]),
            ..Extra::default()
        })),
        (14, 15, Y, Y, "arch", None),
        (14, 15, Y, Y, "cave_entrance", Some(Extra {
            replacements: build_replacements(&[
                (r"^[Jj]jaskyňa\b *", ""),
                (r"\b[Jj]jaskyňa$", "j."),
                (r"\b[Pp]riepasť\b", "p."),
            ]),
            ..Extra::default()
        })),
        (14, 15, Y, Y, "spring", Some(Extra { replacements: spring_replacements.clone(), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, Y, "refitted_spring", Some(Extra { replacements: spring_replacements.clone(), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, Y, "drinking_spring", Some(Extra { replacements: spring_replacements.clone(), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, Y, "not_drinking_spring", Some(Extra { replacements: spring_replacements.clone(), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, Y, "refitted_drinking_spring", Some(Extra { replacements: spring_replacements.clone(), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, Y, "refitted_not_drinking_spring", Some(Extra { replacements: spring_replacements.clone(), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, Y, "hot_spring", Some(Extra { replacements: spring_replacements.clone(), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, Y, "waterfall", Some(Extra {
            replacements: build_replacements(&[
                (r"^[Vv]odopád\b *", ""),
                (r"\b[Vv]odopád$", "vdp."),
            ]),
            ..Extra::default()
        })), // { font: { fill: colors.waterLabel } }
        (14, 15, N, N, "drinking_water", None), // { font: { fill: colors.waterLabel } }
        (14, 15, N, N, "water_point", Some(Extra { icon: Some("drinking_water"), ..Extra::default() })), // { font: { fill: colors.waterLabel } }
        (14, 15, N, N, "water_well", None), // { font: { fill: colors.waterLabel } }
        (14, 15, Y, N, "monument", None),
        (14, 15, Y, Y, "viewpoint", Some(Extra {
            replacements: build_replacements(&[
                (r"^[Vv]yhliadka\b *", ""),
                (r"\b[Vv]yhliadka$", "vyhl."),
            ]),
            ..Extra::default()
        })),
        (14, 15, Y, N, "mine", Some(Extra { icon: Some("mine"), ..Extra::default() })),
        (14, 15, Y, N, "adit", Some(Extra { icon: Some("mine"), ..Extra::default() })),
        (14, 15, Y, N, "mineshaft", Some(Extra { icon: Some("mine"), ..Extra::default() })),
        (14, 15, Y, N, "disused_mine", None),
        (14, 15, Y, N, "hotel", Some(Extra {
            replacements: build_replacements(&[(r"^[Hh]otel\b *", "")]),
            ..Extra::default()
        })),
        (14, 15, Y, N, "chalet", Some(Extra {
            replacements: build_replacements(&[
                (r"^[Cc]hata\b *", ""),
                (r"\b[Cc]hata$", "ch."),
            ]),
            ..Extra::default()
        })),
        (14, 15, Y, N, "hostel", None),
        (14, 15, Y, N, "motel", Some(Extra {
            replacements: build_replacements(&[(r"^[Mm]otel\b *", "")]),
            ..Extra::default()
        })),
        (14, 15, Y, N, "guest_house", None),
        (14, 15, Y, N, "apartment", None),
        (14, 15, Y, N, "wilderness_hut", None),
        (14, 15, Y, N, "alpine_hut", None),
        (14, 15, Y, N, "camp_site", None),
        (14, 15, N, N, "attraction", None),
        (14, 15, N, N, "hospital", Some(Extra {
            replacements: build_replacements(&[(r"^[Nn]emocnica\b", "Nem.")]),
            ..Extra::default()
        })),
        (14, NN, N, N, "townhall", Some(Extra {
            replacements: chapel_replacements.clone(),
            ..Extra::default()
        })),
        (14, 15, N, N, "chapel", None),
        (14, 15, N, N, "church", Some(Extra {
            replacements: church_replacements.clone(),
            ..Extra::default()
        })), // { font: { dy: -13 } }
        (14, 15, N, N, "cathedral", Some(Extra {
            replacements: church_replacements.clone(),
            icon: Some("church"),
            ..Extra::default()
        })), // { font: { dy: -13 } }
        (14, 15, N, N, "synagogue", None),
        (14, 15, N, N, "mosque", None), // { font: { dy: -13 } }
        (14, 15, Y, N, "tower_observation", None),
        (14, 15, Y, N, "archaeological_site", None),
        (14, 15, N, N, "station", None),
        (14, 15, N, N, "halt", Some(Extra { icon: Some("station"), ..Extra::default() })),
        (14, 15, N, N, "bus_station", None),
        (14, 15, N, N, "water_park", None),
        (14, 15, N, N, "museum", None),
        (14, 15, N, N, "manor", None),
        (14, 15, N, N, "free_flying", None),
        (14, 15, N, N, "forester's_lodge", None),
        (14, 15, N, N, "horse_riding", None),
        (14, 15, N, N, "golf_course", None),
        // (14, 14, N, N, "recycling", None), // { font: { fill: colors.areaLabel }, icon: null } // has no icon yet - render as area name
        (15, NN, Y, N, "guidepost_noname", Some(Extra { icon: Some("guidepost_x"), ..Extra::default() })),
        (15, 15, Y, Y, "saddle", None), // { font: { size: 13, dy: -8 } }
        (15, 16, N, N, "ruins", None),
        (15, 16, N, N, "chimney", None),
        (15, 16, N, N, "fire_station", Some(Extra {
            replacements: build_replacements(&[(r"^([Hh]asičská zbrojnica|[Pp]ožiarná stanica)\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "community_centre", Some(Extra {
            replacements: build_replacements(&[(r"\b[Cc]entrum voľného času\b", "CVČ")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "police", Some(Extra {
            replacements: build_replacements(&[(r"^[Pp]olícia\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "office", None),           // information=office
        (15, 16, N, N, "hunting_stand", None),
        (15, 16, Y, N, "shelter", None),
        // (15, 16, Y, N, 'shopping_cart', None),
        (15, 16, Y, N, "lean_to", None),
        (15, 16, Y, N, "public_transport", None),
        (15, 16, Y, N, "picnic_shelter", None),
        (15, 16, Y, N, "basic_hut", None),
        (15, 16, Y, N, "weather_shelter", None),
        (15, 16, N, N, "pharmacy", Some(Extra {
            replacements: build_replacements(&[(r"^[Ll]ekáreň\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "cinema", Some(Extra {
            replacements: build_replacements(&[(r"^[Kk]ino\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "theatre", Some(Extra {
            replacements: build_replacements(&[(r"^[Dd]ivadlo\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "memorial", Some(Extra {
            replacements: build_replacements(&[(r"^[Pp]amätník\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "pub", None),
        (15, 16, N, N, "cafe", Some(Extra {
            replacements: build_replacements(&[(r"^[Kk]aviareň\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "bar", None),
        (15, 16, N, N, "restaurant", Some(Extra {
            replacements: build_replacements(&[(r"^[Rr]eštaurácia\b *", "")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "convenience", None),
        (15, 16, N, N, "supermarket", None),
        (15, 16, N, N, "fast_food", None),
        (15, 16, N, N, "confectionery", None),
        (15, 16, N, N, "pastry", Some(Extra { icon: Some("confectionery"), ..Extra::default() })),
        (15, 16, N, N, "fuel", None),
        (15, 16, N, N, "post_office", None),
        (15, 16, N, N, "bunker", None),
        (15, NN, N, N, "mast_other", None),
        (15, NN, N, N, "tower_other", None),
        (15, NN, N, N, "tower_communication", None),
        (
            15,
            NN,
            N,
            N,
            "mast_communication",
            Some(Extra { icon: Some("tower_communication"), ..Extra::default() }),
        ),
        (15, 16, N, N, "tower_bell_tower", None),
        (15, 16, N, N, "water_tower", None),
        (15, 16, N, N, "bus_stop", None),
        (15, 16, N, N, "sauna", None),
        (15, 16, N, N, "taxi", None),
        (15, 16, N, N, "bicycle", None),
        (15, 15, N, Y, "tree_protected", None), // { font: { fill: hsl(120, 100, 31) } }
        (15, 15, N, Y, "tree", None),
        (15, 16, N, N, "bird_hide", None),
        (15, 16, N, N, "dam", None), // { font: { fill: colors.waterLabel } }
        (15, 16, N, N, "school", Some(Extra { replacements: school_replacements.clone(), ..Extra::default() })),
        (15, 16, N, N, "college", Some(Extra { replacements: college_replacements.clone(), ..Extra::default() })),
        (15, 16, N, N, "university", Some(Extra { replacements: university_replacements.clone(), ..Extra::default() })),
        (15, 16, N, N, "kindergarten", Some(Extra {
            replacements: build_replacements(&[(r"[Mm]atersk(á|ou) [Šš]k[oô]lk?(a|ou)", "MŠ")]),
            ..Extra::default()
        })),
        (15, 16, N, N, "climbing", None),
        (15, 16, N, N, "shooting", None),
        (16, 17, N, Y, "rock", None),
        (16, 17, N, Y, "stone", None),
        (16, 17, N, Y, "sinkhole", None),
        (16, 17, N, N, "building", None),
        (16, 17, N, N, "weir", None), // { font: { fill: colors.waterLabel } },
        (16, 17, N, N, "miniature_golf", None),
        (16, 17, N, N, "soccer", None),
        (16, 17, N, N, "tennis", None),
        (16, 17, N, N, "basketball", None),
        (16, NN, Y, N, "guidepost_noname", Some(Extra { icon: Some("guidepost_x"), ..Extra::default() })),
        (16, NN, Y, N, "route_marker", Some(Extra { icon: Some("guidepost_x"), ..Extra::default() })),
        (16, NN, N, N, "picnic_table", None),
        (16, NN, N, N, "outdoor_seating", None),
        (16, 17, N, N, "picnic_site", None),
        (16, 16, N, N, "board", None),
        (16, 17, N, N, "map", None),
        (16, 17, N, N, "artwork", None),
        (16, 17, N, N, "fountain", None), // { font: { fill: colors.waterLabel } }],
        (16, NN, N, N, "watering_place", None), // { font: { fill: colors.waterLabel } }],
        (16, NN, N, N, "feeding_place", Some(Extra { icon: Some("manger"), ..Extra::default() })),
        (16, NN, N, N, "game_feeding", Some(Extra { icon: Some("manger"), ..Extra::default() })),
        (16, 17, N, N, "playground", Some(Extra {
            replacements: build_replacements(&[(r"^[Dd]etské ihrisko\b", "")]),
            ..Extra::default()
        })),
        (16, 17, N, N, "water_works", None), // { font: { fill: colors.waterLabel } },
        (16, 17, N, N, "reservoir_covered", Some(Extra { icon: Some("water_works"), ..Extra::default() })), // { font: { fill: colors.waterLabel } },
        (16, 17, N, N, "pumping_station", Some(Extra { icon: Some("water_works"), ..Extra::default() })), // { font: { fill: colors.waterLabel } },
        (16, 17, N, N, "wastewater_plant", Some(Extra { icon: Some("water_works"), ..Extra::default() })), // { font: { fill: colors.waterLabel } },
        (16, 17, N, N, "cross", None),
        (17, 18, N, N, "boundary_stone", None),
        (17, 18, N, N, "marker", Some(Extra { icon: Some("boundary_stone"), ..Extra::default() })),
        (17, 18, N, N, "wayside_shrine", None),
        (17, 18, N, N, "cross", None), // NOTE cross is also on lower zoom
        (17, 18, N, N, "wayside_cross", Some(Extra { icon: Some("cross"), ..Extra::default() })), // NOTE cross is also on lower zoom
        (17, 18, N, N, "tree_shrine", Some(Extra { icon: Some("cross"), ..Extra::default() })), // NOTE cross is also on lower zoom
        (17, NN, N, N, "firepit", None),
        (17, NN, N, N, "toilets", None),
        (17, NN, N, N, "bench", None),
        (17, 18, N, N, "beehive", None),
        (17, 18, N, N, "apiary", Some(Extra { icon: Some("beehive"), ..Extra::default() })),
        (17, NN, N, N, "lift_gate", None),
        (17, NN, N, N, "swing_gate", Some(Extra { icon: Some("lift_gate"), ..Extra::default() })),
        (17, NN, N, N, "ford", None),
        (17, 19, N, N, "parking", None), // { font: { dy: -8, fill: colors.areaLabel, size: 10, haloOpacity: 0.5 } },
        (18, 19, N, N, "building_ruins", Some(Extra { icon: Some("ruins"), ..Extra::default() })),
        (18, 19, N, N, "post_box", None),
        (18, 19, N, N, "telephone", None),
        (18, NN, N, N, "gate", None),
        (18, NN, N, N, "waste_disposal", None),
        (19, NN, N, N, "waste_basket", None),
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

    let rows = client
        .query(&sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    let mut to_label = Vec::<(crate::point::Point, f64, String)>::new();

    for row in rows {
        let typ: &str = row.get("type");

        let def = POIS.get(typ);

        let Some(def) = def else {
            continue;
        };

        if def.min_zoom > zoom {
            continue;
        }

        let geom: Point = row.get("geometry");

        let point = geom.project(ctx);

        let surface = svg_cache.get(&format!(
            "images/{}.svg",
            def.extra
                .as_ref()
                .map(|e| e.icon)
                .flatten()
                .unwrap_or_else(|| typ)
        ));

        let rect = surface.extents().unwrap();

        let x = (point.x - rect.width() / 2.0).round();

        let y = (point.y - rect.height() / 2.0).round();

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
            let dx = if i == 0 { 0.0 } else { 10.0 * f64::sin(a) };
            let dy = if i == 0 { 0.0 } else { 10.0 * f64::cos(a) };

            let bbox = BBox {
                min_x: x + dx,
                min_y: y + dy,
                max_x: x + dx + rect.width(),
                max_y: y + dy + rect.height(),
            };

            if collision.collides(bbox) {
                continue;
            }

            to_label.push((
                crate::point::Point {
                    x: point.x + dx,
                    y: point.y + dy,
                },
                rect.height() / 2.0,
                row.get("n"),
            ));

            collision.add(bbox);

            context.set_source_surface(surface, x + dx, y + dy).unwrap();

            context.paint().unwrap();

            break;
        }
    }

    for (point, d, name) in to_label.into_iter() {
        let text_options = TextOptions {
            valign_by_placement: true,
            placements: &[-d - 6.0, d - 3.0],
            ..TextOptions::default()
        };

        let drawn = draw_text(context, collision, point, &name, &text_options);

        if !drawn {
            continue;
        }
    }
}
