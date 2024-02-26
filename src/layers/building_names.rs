use crate::{
    collision::Collision,
    ctx::Ctx,
    draw::{
        draw::Projectable,
        text::{draw_text, TextOptions, DEFAULT_PLACEMENTS},
    },
};
use postgis::ewkb::Point;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    let sql = "
        SELECT osm_buildings.name, ST_Centroid(osm_buildings.geometry) AS geometry
            FROM osm_buildings
            LEFT JOIN osm_landusages USING (osm_id)
            LEFT JOIN osm_feature_polys USING (osm_id)
            LEFT JOIN osm_features USING (osm_id)
            LEFT JOIN osm_place_of_worships USING (osm_id)
            LEFT JOIN osm_sports USING (osm_id)
            LEFT JOIN osm_ruins USING (osm_id)
            LEFT JOIN osm_towers USING (osm_id)
            LEFT JOIN osm_shops USING (osm_id)
            WHERE
            osm_buildings.geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
                AND osm_buildings.type <> 'no'
                AND osm_landusages.osm_id IS NULL
                AND osm_feature_polys.osm_id IS NULL
                AND osm_features.osm_id IS NULL
                AND osm_place_of_worships.osm_id IS NULL
                AND osm_sports.osm_id IS NULL
                AND osm_ruins.osm_id IS NULL
                AND osm_towers.osm_id IS NULL
                AND osm_shops.osm_id IS NULL
            ORDER BY osm_buildings.osm_id";

    let buffer = ctx.meters_per_pixel() * 1024.0;

    for row in &client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .unwrap()
    {
        draw_text(
            context,
            collision,
            row.get::<_, Point>("geometry").project(ctx),
            row.get("name"),
            &TextOptions {
                placements: DEFAULT_PLACEMENTS,
                ..TextOptions::default()
            },
        );
    }
}
