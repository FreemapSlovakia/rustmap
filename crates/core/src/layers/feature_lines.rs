use crate::{
    SvgCache,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{line_pattern::draw_line_pattern_scaled, path_geom::path_line_string},
    projectable::{TileProjectable, geometry_line_string},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, svg_cache: &mut SvgCache) {
    let context = ctx.context;
    let zoom = ctx.zoom;

    let sql = "SELECT geometry, type FROM osm_feature_lines
          WHERE
            type IN ('weir', 'dam', 'tree_row') AND
            geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)";

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(8.0)).as_params())
        .expect("db data");

    for row in rows {
        let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        context.save().expect("context saved");

        match row.get("type") {
            "weir" => {
                if zoom >= 16 {
                    context.set_dash(&[9.0, 3.0], 0.0);
                    context.set_source_color(colors::DAM_LINE);
                    context.set_line_width(3.0);
                    path_line_string(context, &geom);
                    context.stroke().unwrap();
                }
            }
            "dam" => {
                if zoom >= 16 {
                    context.set_source_color(colors::DAM_LINE);
                    context.set_line_width(3.0);
                    path_line_string(context, &geom);
                    context.stroke().unwrap();
                }
            }
            "tree_row" => {
                draw_line_pattern_scaled(
                    ctx,
                    &geom,
                    0.8,
                    (2.0 + (zoom as f64 - 15.0).exp2()) / 4.5,
                    svg_cache.get("tree2.svg"),
                );
            }
            _ => panic!("unexpected type"),
        }

        context.restore().expect("context restores");
    }
}
