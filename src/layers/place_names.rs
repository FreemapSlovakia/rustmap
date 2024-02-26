use crate::{
    collision::Collision,
    ctx::Ctx,
    draw::{
        draw::Projectable,
        text::{self, draw_text, TextOptions},
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

    let zoom = ctx.zoom;

    let sql = &format!(
        "SELECT name, type, geometry
            FROM osm_places
            WHERE {} AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)
            ORDER BY z_order DESC, population DESC, osm_id",
        match zoom {
            8 => "type = 'city'",
            9..=10 => "(type = 'city' OR type = 'town')",
            11 => "(type = 'city' OR type = 'town' OR type = 'village')",
            12.. => "type <> 'locality'",
            _ => panic!("unsupported zoom"),
        }
    );

    let scale = 2.5 * 1.2f64.powf(zoom as f64);

    let buffer = ctx.meters_per_pixel() * 1024.0;

    for row in &client
        .query(sql, &[min_x, min_y, max_x, max_y, &buffer])
        .unwrap()
    {
        let (size, uppercase, halo_width) = match (zoom, row.get("type")) {
            (6.., "city") => (1.2, true, 2.0),
            (9.., "town") => (0.8, true, 2.0),
            (11.., "village") => (0.55, true, 1.5),
            (12.., "hamlet" | "allotments" | "suburb") => (0.50, false, 1.5),
            (14.., "isolated_dwelling" | "quarter") => (0.45, false, 1.5),
            (15.., "neighbourhood") => (0.40, false, 1.5),
            (16.., "farm" | "borough" | "square") => (0.35, false, 1.5),
            _ => continue,
        };

        draw_text(
            context,
            collision,
            row.get::<_, Point>("geometry").project(ctx),
            row.get("name"),
            &TextOptions {
                size: size * scale,
                halo_width,
                halo_opacity: 0.9,
                uppercase,
                narrow: true,
                alpha: if zoom <= 14 { 1.0 } else { 0.5 },
                weight: pango::Weight::Bold,
                letter_spacing: 1.0,
                placements: text::DEFAULT_PLACEMENTS,
                ..TextOptions::default()
            },
        );
    }
}
