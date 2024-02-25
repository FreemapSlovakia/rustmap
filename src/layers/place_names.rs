use crate::{ctx::Ctx, draw::draw::Projectable};
use pango::AttrInt;
use pangocairo::{
    functions::{create_layout, layout_path},
    pango::{AttrList, FontDescription},
};
use postgis::ewkb::Point;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let sql = &format!(
        "SELECT name, type, geometry
            FROM osm_places
            WHERE {} AND geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), {})
            ORDER BY z_order DESC, population DESC, osm_id",
        match zoom {
            8 => "type = 'city'",
            9..=10 => "(type = 'city' OR type = 'town')",
            11 => "(type = 'city' OR type = 'town' OR type = 'village')",
            12.. => "type <> 'locality'",
            _ => panic!("unsupported zoom"),
        },
        ctx.meters_per_pixel() * 100.0
    );

    let scale = 2.5 / 1.333 * 1.2f64.powf(zoom as f64);

    for row in &client.query(sql, &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: Point = row.get("geometry");

        let typ: &str = row.get("type");

        let layout = create_layout(context);

        let (size, uppercase) = match (zoom, typ) {
            (6.., "city") => (1.2 * scale, true),
            (9.., "town") => (0.8 * scale, true),
            (11.., "village") => (0.55 * scale, true),
            (12.., "hamlet" | "allotments" | "suburb") => (0.50 * scale, false),
            (14.., "isolated_dwelling" | "quarter") => (0.45 * scale, false),
            (15.., "neighbourhood") => (0.40 * scale, false),
            (16.., "farm" | "borough" | "square") => (0.35 * scale, false),
            _ => continue,
        };

        let mut font_description = FontDescription::new();
        font_description.set_family("PT Sans Narrow");
        font_description.set_weight(pango::Weight::Bold);
        font_description.set_size((pango::SCALE as f64 * size) as i32);

        layout.set_font_description(Some(&font_description));

        let original_text: &str = row.get("name");

        let uppercase_text;

        let text = if uppercase {
            uppercase_text = original_text.to_uppercase();
            &uppercase_text
        } else {
            original_text
        };

        layout.set_wrap(pango::WrapMode::Word);
        layout.set_alignment(pango::Alignment::Center);
        layout.set_line_spacing(0.4);
        layout.set_width(133 * pango::SCALE);

        layout.set_text(text);

        let attr_list = AttrList::new();

        attr_list.insert(AttrInt::new_letter_spacing(pango::SCALE));

        layout.set_attributes(Some(&attr_list));

        let p = geom.project(ctx);

        let size = layout.pixel_size();

        context.move_to(p.x - 133 as f64 / 2.0, p.y - size.1 as f64 / 2.0);

        layout_path(context, &layout);

        context.push_group();

        context.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        context.set_dash(&[], 0.0);
        context.set_line_width(3.0);

        context.stroke_preserve().unwrap();

        context.set_source_rgb(0.0, 0.0, 0.0);

        context.fill().unwrap();

        context.pop_group_to_source().unwrap();

        context
            .paint_with_alpha(if zoom <= 14 { 1.0 } else { 0.5 })
            .unwrap();
    }
}
