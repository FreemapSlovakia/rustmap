use crate::{
    collision::Collision,
    colors::{self},
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        text_on_line::{TextOnLineOptions, text_on_line},
    },
    projectable::{TileProjectable, geometry_line_string},
};
use pangocairo::pango::Style;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let zoom = ctx.zoom;

    let sql = format!(
        "WITH merged AS (
            SELECT
                ST_LineMerge(ST_Collect(ST_Segmentize(ST_Simplify(geometry, 24), 200))) AS geometry,
                name, type, MIN(osm_id) AS osm_id
            FROM osm_waterways
            WHERE name <> '' {}AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
            GROUP BY name, type
        )
        SELECT name, (ST_Dump(ST_CollectionExtract(geometry, 2))).geom AS geometry
        FROM merged ORDER BY osm_id, type", // TODO order by type - river 1st
        if zoom < 14 { "AND type = 'river' " } else { "" }
    );

    let rows = client
        .query(&sql, &ctx.bbox_query_params(Some(1024.0)).as_params())
        .expect("db data");

    let options = TextOnLineOptions {
        flo: FontAndLayoutOptions {
            style: Style::Italic,
            letter_spacing: 2.0,
            ..FontAndLayoutOptions::default()
        },
        repeat_distance: Some(200.0),
        spacing: 200.0,
        color: colors::WATER_LABEL,
        halo_color: colors::WATER_LABEL_HALO,
        ..TextOnLineOptions::default()
    };

    for row in rows {
        let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        let name: &str = row.get("name");

        text_on_line(ctx.context, &geom, name, Some(collision), &options);
    }
}
