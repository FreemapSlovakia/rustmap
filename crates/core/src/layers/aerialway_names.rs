use crate::{
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        offset_line::offset_line_string,
        text_on_line::{Align, Distribution, Repeat, TextOnLineOptions, draw_text_on_line},
    },
    projectable::{TileProjectable, geometry_line_string},
};
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let sql = concat!(
        "SELECT geometry, name FROM osm_aerialways ",
        "WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)"
    );

    let rows = client
        .query(sql, &ctx.bbox_query_params(Some(512.0)).as_params())
        .expect("db data");

    let options = TextOnLineOptions {
        distribution: Distribution::Align {
            align: Align::Center,
            repeat: Repeat::Spaced(200.0),
        },
        color: colors::BLACK,
        ..TextOnLineOptions::default()
    };

    for row in rows {
        let name: &str = row.get("name");

        let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        let geom = offset_line_string(&geom, 10.0);

        draw_text_on_line(ctx.context, &geom, name, Some(collision), &options);
    }
}
