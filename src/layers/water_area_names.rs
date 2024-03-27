use crate::{
    bbox::BBox, collision::Collision, colors, ctx::Ctx, draw::{
        draw::Projectable,
        text::{self, draw_text, TextOptions},
    }
};
use pangocairo::pango::Style;
use postgis::ewkb::Point;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let Ctx {
        context,
        bbox: BBox { min_x, min_y, max_x, max_y },
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let sql = "
        SELECT
            REGEXP_REPLACE(osm_waterareas.name, '[Vv]odná [Nn]ádrž\\M', 'v. n.') AS name,
            ST_Centroid(osm_waterareas.geometry) AS geometry
        FROM
            osm_waterareas LEFT JOIN osm_feature_polys USING (osm_id)
        WHERE
            osm_waterareas.geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $6)
            AND osm_feature_polys.osm_id IS NULL
            AND osm_waterareas.type <> 'riverbank'
            AND osm_waterareas.water NOT IN ('river', 'stream', 'canal', 'ditch')
            AND ($5 >= 17 OR osm_waterareas.area > 800000 / POWER(2, (2 * ($5 - 10))))
        ";

    let buffer = ctx.meters_per_pixel() * 1024.0;

    for row in &client
        .query(sql, &[min_x, min_y, max_x, max_y, &(zoom as i32), &buffer])
        .unwrap()
    {
        draw_text(
            context,
            collision,
            row.get::<_, Point>("geometry").project(ctx),
            row.get("name"),
            &TextOptions {
                color: colors::WATER_LABEL,
                halo_color: colors::WATER_LABEL_HALO,
                style: Style::Italic,
                placements: text::DEFAULT_PLACEMENTS,
                ..TextOptions::default()
            },
        );
    }
}
