use crate::{
    bbox::BBox,
    collision::Collision,
    colors,
    ctx::Ctx,
    draw::{
        draw::Projectable,
        offset_line::offset_line,
        text_on_line::{TextOnLineOptions, text_on_line},
    },
};
use geo::Coord;
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client, collision: &mut Collision<f64>) {
    let Ctx {
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        ..
    } = ctx;

    let sql = concat!(
        "SELECT geometry, name FROM osm_aerialways ",
        "WHERE geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)"
    );

    let buffer = ctx.meters_per_pixel() * 512.0;

    let rows = client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .expect("db data");

    let options = TextOnLineOptions {
        repeat_distance: Some(200.0),
        spacing: 200.0,
        color: colors::BLACK,
        ..TextOnLineOptions::default()
    };

    for row in rows {
        let geom: LineString = row.get("geometry");

        let name: &str = row.get("name");

        let projected: Vec<Coord> = geom.points.iter().map(|p| p.project(ctx)).collect();

        let points = offset_line(projected, 10.0);

        text_on_line(ctx, points, name, Some(collision), &options);
    }
}
