use crate::bbox::BBox;
use crate::colors::{Color, ContextExt};
use crate::draw::markers_on_path::draw_markers_on_path;
use crate::{colors, ctx::Ctx, draw::draw::draw_line};
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        bbox:
            BBox {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        context,
        ..
    } = ctx;

    let zoom = ctx.zoom;

    // TODO no roads on zoom 7 and lower

    let table = match zoom {
        ..=9 => "osm_roads_gen0",
        10..=11 => "osm_roads_gen1",
        12.. => "osm_roads",
    };

    let query = format!("SELECT {table}.geometry, {table}.type, tracktype, class, service, bridge, tunnel, oneway, bicycle, foot,
            power(0.666, greatest(0, trail_visibility - 1))::DOUBLE PRECISION AS trail_visibility,
            osm_route_members.member IS NOT NULL AS is_in_route
        FROM {table} LEFT JOIN osm_route_members ON osm_route_members.type = 1 AND osm_route_members.member = {table}.osm_id
        WHERE {table}.geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
        ORDER BY z_order, CASE WHEN {table}.type = 'rail' AND service IN ('', 'main') THEN 2 ELSE 1 END, {table}.osm_id", table = table);

    let apply_highway_defaults = |width: f64| {
        context.set_dash(&[], 0.0);
        context.set_source_color(colors::TRACK);
        context.set_line_join(cairo::LineJoin::Round);
        context.set_line_width(width);
    };

    let apply_glow_defaults = |width: f64| {
        context.set_source_color(colors::GLOW);
        context.set_dash(&[], 0.0);
        context.set_line_join(cairo::LineJoin::Round);
        context.set_line_width(width);
    };

    let highway_width_coef = || 1.5f64.powf(8.6f64.max(zoom as f64) - 8.0);

    let rows = &client.query(&query, &[min_x, min_y, max_x, max_y]).unwrap();

    let ke = || match zoom {
        12 => 0.66,
        13 => 0.75,
        14.. => 1.00,
        _ => 0.00,
    };

    let mut cache = ctx.svg_cache.borrow_mut();

    // TODO lazy
    let arrow = cache.get("images/highway-arrow.svg");

    let rect = arrow.extents().unwrap();

    for row in rows {
        let geom: LineString = row.get("geometry");

        let typ: &str = row.get("type");

        let class: &str = row.get("class");

        let draw = || {
            draw_line(ctx, geom.points.iter());

            context.stroke().unwrap();
        };

        match (zoom, class, typ) {
            (..=11, _, _) => (),
            (14.., "highway", "footway" | "pedestrian" | "platform" | "steps") => {
                apply_glow_defaults(1.0);
                draw();
            }
            (14.., "highway", "via_ferrata") => {
                apply_glow_defaults(3.0);
                context.set_source_rgb(0.0, 0.0, 0.0);
                context.set_dash(&[0.0, 4.0, 4.0, 0.0], 0.0);
                draw();

                apply_glow_defaults(1.0);
                draw();
            }
            (12.., "highway", "path")
                if row.get::<_, &str>("bicycle") != "designated"
                    && (zoom > 12 || row.get("is_in_route")) =>
            {
                apply_glow_defaults(1.0);
                // TODO strokeOpacity="[trail_visibility]"
                draw();
            }
            (12.., "highway", _)
                if typ == "track"
                    && (zoom > 12
                        || row.get("is_in_route")
                        || row.get::<_, &str>("tracktype") == "grade1")
                    || typ == "service" && row.get::<_, &str>("service") != "parking_aisle"
                    || ["escape", "corridor", "bus_guideway"].contains(&typ) =>
            {
                apply_glow_defaults(ke() * 1.2);
                // TODO strokeOpacity="[trail_visibility]"
                draw();
            }
            (14.., "highway", _) if typ == "raceway" || (typ == "track" && class == "leisure") => {
                apply_glow_defaults(1.2);
                draw();
            }
            (13.., "highway", "bridleway")
                if typ == "raceway" || (typ == "track" && class == "leisure") =>
            {
                apply_glow_defaults(1.2);
                context.set_source_color(colors::BRIDLEWAY2);
                // strokeOpacity="[trail_visibility]"
                draw();
            }
            (_, "highway", "motorway" | "trunk") => {
                apply_highway_defaults(4.0);
                draw();
            }
            (_, "highway", "primary" | "motorway_link" | "trunk_link") => {
                apply_highway_defaults(3.666);
                draw();
            }
            (_, "highway", "primary_link" | "secondary" | "construction") => {
                apply_highway_defaults(3.333);
                draw();
            }
            (_, "highway", "secondary_link" | "tertiary" | "tertiary_link") => {
                apply_highway_defaults(3.0);
                draw();
            }
            (14.., "highway", "living_street" | "residential" | "unclassified" | "road") => {
                apply_highway_defaults(2.5);
                draw();
            }
            (14.., "highway", "piste") => {
                apply_highway_defaults(2.2);
                context.set_dash(&[6.0, 2.0], 0.0);
                context.set_source_color(colors::PISTE2);
                draw();
            }
            _ => (),
        }
    }

    for row in rows {
        let geom: LineString = row.get("geometry");

        let typ: &str = row.get("type");

        let class: &str = row.get("class");

        let service: &str = row.get("service");

        let draw = || {
            draw_line(ctx, geom.points.iter());

            context.stroke().unwrap();
        };

        let draw_bridges_tunnels = |width: f64| {
            if row.get::<_, i16>("bridge") > 0 {
                context.save().unwrap();
                context.set_dash(&[], 0.0);
                context.set_source_rgb(0.0, 0.0, 0.0);

                context.push_group();

                context.set_line_cap(cairo::LineCap::Butt);
                context.set_line_width(width + 2.0);
                draw();
                context.stroke().unwrap();

                context.set_line_cap(cairo::LineCap::Square);
                context.set_operator(cairo::Operator::Clear);
                context.set_line_width(width);
                draw();
                context.stroke().unwrap();

                context.pop_group_to_source().unwrap();
                context.paint().unwrap();

                context.restore().unwrap();
            }

            if row.get::<_, i16>("tunnel") > 0 {
                context.set_dash(&[], 0.0);
                context.set_line_width(width + 1.0);

                context.set_source_rgba(0.8, 0.8, 0.8, 0.8);
                draw();
                context.stroke().unwrap();

                context.save().unwrap();
                context.set_dash(&[3.0, 3.0], 0.0);
                context.set_source_rgba(0.0, 0.0, 0.0, 0.5);

                context.push_group();

                context.set_line_cap(cairo::LineCap::Butt);
                context.set_line_width(width + 2.0);
                draw();
                context.stroke().unwrap();

                context.set_line_cap(cairo::LineCap::Square);
                context.set_operator(cairo::Operator::Clear);
                context.set_line_width(width + 0.8);
                draw();
                context.stroke().unwrap();

                context.pop_group_to_source().unwrap();
                context.paint().unwrap();

                context.restore().unwrap();
            }
        };

        let draw_rail =
            |color: Color, weight: f64, sleeper_weight: f64, spacing: f64, glow_width: f64| {
                context.set_line_join(cairo::LineJoin::Round);

                let gw = weight + glow_width * 2.0;

                let sgw = sleeper_weight + glow_width * 2.0;

                context.set_source_color(colors::RAIL_GLOW);
                context.set_dash(&[], 0.0);
                context.set_line_width(gw);
                draw_line(ctx, geom.points.iter());
                context.stroke().unwrap();

                context.set_dash(&[0.0, (spacing - gw) / 2.0, gw, (spacing - gw) / 2.0], 0.0);
                context.set_line_width(sgw);
                draw_line(ctx, geom.points.iter());
                context.stroke().unwrap();

                context.set_source_color(color);
                context.set_dash(&[], 0.0);
                context.set_line_width(weight);
                draw_line(ctx, geom.points.iter());
                context.stroke().unwrap();

                context.set_dash(
                    &[
                        0.0,
                        (spacing - weight) / 2.0,
                        weight,
                        (spacing - weight) / 2.0,
                    ],
                    0.0,
                );
                context.set_line_width(sleeper_weight);
                draw_line(ctx, geom.points.iter());
                context.stroke().unwrap();

                draw_bridges_tunnels(sleeper_weight + glow_width);
            };

        match (zoom, class, typ) {
            (14.., _, "pier") => {
                apply_highway_defaults(2.0);
                context.set_source_color(colors::PIER);
                draw();
            }
            (12.., "railway", "rail") if ["main", ""].contains(&service) => {
                draw_rail(colors::RAIL, 1.5, 5.0, 9.5, 1.0);
            }
            (13.., "railway", _)
                if ["light_rail", "tram"].contains(&typ)
                    || typ == "rail" && service != "main" && !service.is_empty() =>
            {
                draw_rail(colors::TRAM, 1.0, 4.5, 9.5, 1.0);
            }
            (
                13..,
                "railway",
                "miniature" | "monorail" | "funicular" | "narrow_gauge" | "subway",
            ) => {
                draw_rail(colors::TRAM, 1.0, 4.5, 7.5, 1.0);
            }
            (14.., "railway", "construction" | "disused" | "preserved") => {
                draw_rail(colors::RAILWAY_DISUSED, 1.0, 4.5, 7.5, 1.0);
            }
            (8..=11, "railway", "rail") if ["main", ""].contains(&service) => {
                let koef = 0.8 * 1.15f64.powf((zoom - 8) as f64);

                draw_rail(
                    colors::RAIL,
                    koef,
                    10.0 / 3.0 * koef,
                    9.5 / 1.5 * koef,
                    0.5 * koef,
                );
            }
            (8..=11, "highway", "motorway" | "trunk" | "motorway_link" | "trunk_link") => {
                apply_highway_defaults(0.8 * highway_width_coef());
                draw();
            }
            (8..=11, "highway", "primary" | "primary_link") => {
                apply_highway_defaults(0.7 * highway_width_coef());
                draw();
            }
            (8..=11, "highway", "secondary" | "secondary_link") => {
                apply_highway_defaults(0.6 * highway_width_coef());
                draw();
            }
            (8..=11, "highway", "tertiary" | "tertiary_link") => {
                apply_highway_defaults(0.5 * highway_width_coef());
                draw();
            }
            (12.., "highway", "motorway" | "trunk") => {
                apply_highway_defaults(2.5);
                context.set_source_color(colors::SUPERROAD);
                draw();

                draw_bridges_tunnels(2.5 + 1.0);
            }
            (12.., "highway", "motorway_link" | "trunk_link") => {
                apply_highway_defaults(1.5 + 2.0 / 3.0);
                context.set_source_color(colors::SUPERROAD);
                draw();

                draw_bridges_tunnels(1.5 + 2.0 / 3.0 + 1.0);
            }
            (12.., "highway", "primary") => {
                apply_highway_defaults(1.5 + 2.0 / 3.0);
                context.set_source_color(colors::ROAD);
                draw();

                draw_bridges_tunnels(1.5 + 2.0 / 3.0 + 1.0);
            }
            (12.., "highway", "primary_link" | "secondary") => {
                apply_highway_defaults(1.5 + 1.0 / 3.0);
                context.set_source_color(colors::ROAD);
                draw();

                draw_bridges_tunnels(1.5 + 1.0 / 3.0 + 1.0);
            }
            (12.., "highway", "construction") => {
                apply_highway_defaults(1.5 + 1.0 / 3.0);
                context.set_source_color(colors::CONSTRUCTION_ROAD_1);
                context.set_dash(&[5.0, 5.0], 0.0);
                draw();

                context.set_source_color(colors::CONSTRUCTION_ROAD_2);
                context.set_dash(&[5.0, 5.0], 5.0);
                draw();
            }
            (12.., "highway", "secondary_link" | "tertiary" | "tertiary_link") => {
                apply_highway_defaults(1.5);
                context.set_source_color(colors::ROAD);
                draw();

                draw_bridges_tunnels(1.5 + 1.0 / 3.0 + 1.0);
            }
            (12..=13, "highway", "living_street" | "residential" | "unclassified" | "road") => {
                apply_highway_defaults(1.0);
                draw();

                draw_bridges_tunnels(1.0 + 1.0);
            }
            (14.., "highway", "living_street" | "residential" | "unclassified" | "road") => {
                apply_highway_defaults(1.0);
                context.set_source_color(colors::ROAD);
                draw();

                draw_bridges_tunnels(1.0 + 1.0);
            }
            (14.., "highway", "water_slide") => {
                apply_highway_defaults(1.5);
                context.set_source_color(colors::WATER_SLIDE);
                draw();

                draw_bridges_tunnels(1.5 + 1.0);
            }
            (14.., "highway", "service") if service == "parking_aisle" => {
                apply_highway_defaults(1.0);
                draw();

                draw_bridges_tunnels(1.0 + 1.0);
            }
            (14.., "highway", _) if typ == "raceway" || typ == "track" && class == "leisure" => {
                apply_highway_defaults(1.2);
                context.set_dash(&[9.5, 1.5], 0.0);
                draw();

                draw_bridges_tunnels(1.2 + 1.0);
            }
            (14.., "highway", "piste") => {
                apply_highway_defaults(1.2);
                context.set_source_color(colors::PISTE);
                context.set_dash(&[9.5, 1.5], 0.0);
                draw();

                draw_bridges_tunnels(1.2 + 1.0);
            }
            (14.., "highway", "footway" | "pedestrian" | "platform") => {
                apply_highway_defaults(1.0);
                context.set_dash(&[4.0, 2.0], 0.0);
                draw();

                draw_bridges_tunnels(1.0 + 1.0);
            }
            (14.., "highway", "steps") => {
                apply_highway_defaults(2.5);
                context.set_dash(&[1.0, 2.0], 2.0);
                draw();
            }
            (12.., "highway", _)
                if typ == "service" && service != "parking_aisle"
                    || ["escape", "corridor", "bus_guideway"].contains(&typ) =>
            {
                let width = ke() * 1.2;

                apply_highway_defaults(width);
                draw();

                draw_bridges_tunnels(width + 1.0);
            }
            (12.., "highway", "path")
                if row.get::<_, &str>("bicycle") == "designated"
                    && row.get::<_, &str>("foot") == "designated"
                    && (zoom > 12 || row.get("is_in_route")) =>
            {
                let width = ke();

                apply_highway_defaults(width);
                context.set_dash(&[4.0, 2.0], 0.0);
                context.set_source_color_a(colors::CYCLEWAY, row.get("trail_visibility"));
                draw();

                draw_bridges_tunnels(width + 1.0);
            }
            (12.., "highway", _)
                if (typ == "cycleway"
                    || typ == "path"
                        && row.get::<_, &str>("bicycle") == "designated"
                        && row.get::<_, &str>("foot") != "designated")
                    && (zoom > 12 || row.get("is_in_route")) =>
            {
                let width = ke();

                apply_highway_defaults(width);
                context.set_dash(&[6.0, 3.0], 0.0);
                context.set_source_color_a(colors::CYCLEWAY, row.get("trail_visibility"));
                draw();

                draw_bridges_tunnels(width + 1.0);
            }
            (12.., "highway", "path")
                if (row.get::<_, &str>("bicycle") != "designated"
                    || row.get::<_, &str>("foot") == "designated")
                    && (zoom > 12 || row.get("is_in_route")) =>
            {
                let width = ke();

                apply_highway_defaults(width);
                context.set_dash(&[3.0, 3.0], 0.0);
                context.set_source_color_a(colors::TRACK, row.get("trail_visibility"));
                draw();

                draw_bridges_tunnels(width + 1.0);
            }
            (12.., "highway", "bridleway") if zoom > 12 || row.get("is_in_route") => {
                let width = ke();

                apply_highway_defaults(width);
                context.set_dash(&[6.0, 3.0], 0.0);
                context.set_source_color_a(colors::BRIDLEWAY, row.get("trail_visibility"));
                draw();

                draw_bridges_tunnels(width + 1.0);
            }
            (12.., "highway", "via_ferrata") if zoom > 12 || row.get("is_in_route") => {
                let width = ke();

                apply_highway_defaults(width);
                context.set_dash(&[4.0, 4.0], 0.0);
                draw();

                draw_bridges_tunnels(width + 1.0);
            }
            (12.., "highway", "track")
                if (zoom > 12
                    || row.get("is_in_route")
                    || row.get::<_, &str>("tracktype") == "grade1") =>
            {
                let width = ke() * 1.2;

                apply_highway_defaults(width);
                context.set_dash(
                    match row.get::<_, &str>("tracktype") {
                        "grade1" => &[],
                        "grade2" => &[8.0, 2.0],
                        "grade3" => &[6.0, 4.0],
                        "grade4" => &[4.0, 6.0],
                        "grade5" => &[2.0, 8.0],
                        _ => &[3.0, 7.0, 7.0, 3.0],
                    },
                    0.0,
                );
                context.set_source_color_a(colors::TRACK, row.get("trail_visibility"));
                draw();

                draw_bridges_tunnels(width + 1.0);
            }

            // <RuleEx minZoom={14} filter="[oneway] <> 0">
            //   <MarkersSymbolizer
            //     file="images/highway-arrow.svg"
            //     spacing={100}
            //     placement="line"
            //     transform="rotate(90 - [oneway] * 90, 0, 0)"
            //   />
            // </RuleEx>
            _ => (),
        };

        let oneway = row.get::<_, i16>("oneway");

        if zoom >= 14 && oneway != 0 {
            draw_line(ctx, geom.points.iter());

            let path = context.copy_path().unwrap();

            context.new_path();

            draw_markers_on_path(&path, 50.0, 100.0, &|x, y, angle| {
                context.save().unwrap();
                context.translate(x, y);
                context.rotate(angle + if oneway < 0 { 180.0 } else { 0.0 });
                context
                    .set_source_surface(arrow, -rect.width() / 2.0, -rect.height() / 2.0)
                    .unwrap();
                context.paint().unwrap();
                context.restore().unwrap();
            });
        }
    }
}
