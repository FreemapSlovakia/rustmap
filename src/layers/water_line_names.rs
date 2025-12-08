use crate::{
    bbox::BBox,
    collision::Collision,
    colors::{self},
    ctx::Ctx,
    draw::{
        create_pango_layout::FontAndLayoutOptions,
        text_on_line::{TextOnLineOptions, text_on_line},
    },
    projectable::Projectable,
};
use geo::Coord;
use pangocairo::pango::Style;
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
        zoom,
        ..
    } = ctx;

    if *zoom < 12 {
        return;
    }

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
        if *zoom < 14 {
            "AND type = 'river' "
        } else {
            ""
        }
    );

    let buffer = ctx.meters_per_pixel() * 1024.0;

    let rows = client
        .query(&sql, &[min_x, min_y, max_x, max_y, &buffer])
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
        let geom: LineString = row.get("geometry");

        let name: &str = row.get("name");

        let projected: Vec<Coord> = geom.points.iter().map(|p| p.project(ctx)).collect();

        text_on_line(ctx, projected, name, Some(collision), &options);
    }
}
