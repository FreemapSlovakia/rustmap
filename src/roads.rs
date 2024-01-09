use crate::colors::ContextExt;
use crate::{colors, ctx::Ctx, draw::draw_line};
use postgis::ewkb::LineString;
use postgres::Client;

pub fn render(ctx: &Ctx, zoom: u32, client: &mut Client) {
    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        context,
        ..
    } = ctx;

    // TODO no roads on zoom 7 and lower

    let table = match zoom {
        ..=9 => "osm_roads_gen0",
        10..=11 => "osm_roads_gen1",
        12.. => "osm_roads",
    };

    let query = format!("SELECT {table}.geometry, {table}.type, tracktype, class, service, bridge, tunnel, oneway, power(0.666, greatest(0, trail_visibility - 1)) AS trail_visibility, bicycle, foot, osm_route_members.member IS NOT NULL AS is_in_route
        FROM {table} LEFT JOIN osm_route_members ON osm_route_members.type = 1 AND osm_route_members.member = {table}.osm_id
        WHERE {table}.geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
        ORDER BY z_order, CASE WHEN {table}.type = 'rail' AND service IN ('', 'main') THEN 2 ELSE 1 END, {table}.osm_id", table = table);

    let apply_highway_defaults = |width: f64| {
        context.set_dash(&[], 0.0);
        context.set_source_color(*colors::TRACK);
        context.set_line_join(cairo::LineJoin::Round);
        context.set_line_width(width);
    };

    let highway_width_coef = || 1.5f64.powf(8.6f64.max(zoom as f64) - 8.0);

    for row in &client
        .query(&query, &[&min_x, &min_y, &max_x, &max_y])
        .unwrap()
    {
        let geom: LineString = row.get("geometry");

        let typ: &str = row.get("type");

        let class: &str = row.get("class");

        let service: &str = row.get("service");

        let draw = || {
            draw_line(&ctx, geom.points.iter());

            context.stroke().unwrap();
        };

        // let draw_road = || {
        //     context.set_line_join(cairo::LineJoin::Round);
        //     draw();
        // };

        let ke = || match zoom {
            12 => 0.66,
            13 => 0.75,
            14.. => 1.00,
            _ => 0.00,
        };

        match (zoom, class, typ) {
            (14.., _, "pier") => {
                apply_highway_defaults(2.0);
                draw();
            }
            (13.., "railway", _)
                if (["light_rail", "tram"].contains(&typ)
                    || typ == "rail" && service != "main" && service != "") =>
            {
                // TODO <Rail color={hsl(0, 0, 20)} weight={1} sleeperWeight={4.5} spacing={9.5} glowWidth={1} />
            }
            (
                13..,
                "railway",
                "miniature" | "monorail" | "funicular" | "narrow_gauge" | "subway",
            ) => {
                // TODO <Rail color={hsl(0, 0, 20)} weight={1} sleeperWeight={4.5} spacing={7.5} glowWidth={1} />
            }
            (14.., "railway", "construction" | "disused" | "preserved") => {
                // TODO <Rail color={hsl(0, 0, 33)} weight={1} sleeperWeight={4.5} spacing={7.5} glowWidth={1} />
            }
            (8..=14, "railway", "rail") if ["main", ""].contains(&service) => {
                let koef = 0.8 * 1.15f64.powf((zoom - 8) as f64);

                // TODO
                // <Rail
                //     color="black"
                //     weight={koef}
                //     sleeperWeight={(10 / 3) * koef}
                //     spacing={(9.5 / 1.5) * koef}
                //     glowWidth={0.5 * koef}
                // />
            }
            (8..=11, _, "motorway" | "trunk" | "motorway_link" | "trunk_link") => {
                apply_highway_defaults(0.8 * highway_width_coef());
                draw();
            }
            (8..=11, _, "primary" | "primary_link") => {
                apply_highway_defaults(0.7 * highway_width_coef());
                draw();
            }
            (8..=11, _, "secondary" | "secondary_link") => {
                apply_highway_defaults(0.6 * highway_width_coef());
                draw();
            }
            (8..=11, _, "tertiary" | "tertiary_link") => {
                apply_highway_defaults(0.5 * highway_width_coef());
                draw();
            }
            (12.., _, "motorway" | "trunk") => {
                apply_highway_defaults(2.5);
                context.set_source_color(*colors::SUPERROAD);
                draw();
                // TODO Road
            }
            (12.., _, "motorway_link" | "trunk_link") => {
                apply_highway_defaults(1.5 + 2.0 / 3.0);
                context.set_source_color(*colors::SUPERROAD);
                draw();
                // TODO Road
            }
            (12.., _, "primary") => {
                context.set_line_width(1.5 + 2.0 / 3.0);
                context.set_source_color(*colors::ROAD);
                draw();
                // TODO Road
            }
            (12.., _, "primary_link" | "secondary") => {
                context.set_line_width(1.5 + 1.0 / 3.0);
                context.set_source_color(*colors::ROAD);
                draw();
                // TODO Road
            }
            (12.., _, "construction") => {
                //   <Road stroke="yellow" strokeWidth={1.5 + 1 / 3} strokeDasharray="5,5" />
                //   <Road stroke="#666" strokeWidth={1.5 + 1 / 3} strokeDasharray="0,5,5,0" />
            }
            (12.., _, "secondary_link" | "tertiary" | "tertiary_link") => {
                apply_highway_defaults(1.5);
                context.set_source_color(*colors::ROAD);
                draw();
                // TODO Road
            }
            (12..=14, _, "living_street" | "residential" | "unclassified" | "road") => {
                apply_highway_defaults(1.0);
                draw();
                // TODO Road
            }
            (15.., _, "living_street" | "residential" | "unclassified" | "road") => {
                apply_highway_defaults(1.0);
                context.set_source_color(*colors::ROAD);
                draw();
                // TODO Road
            }
            (14.., _, "water_slide") => {
                apply_highway_defaults(1.5);
                context.set_source_color(*colors::WATER_SLIDE);
                draw();
                // TODO Road
            }
            (14.., _, "service") if service == "parking_aisle" => {
                apply_highway_defaults(1.0);
                draw();
                // TODO Road
            }
            (14.., _, _) if typ == "raceway" || typ == "track" && class == "leisure" => {
                apply_highway_defaults(1.2);
                context.set_dash(&[9.5, 1.5], 0.0);
                draw();
                // TODO Road
            }
            (14.., _, "piste") => {
                apply_highway_defaults(1.2);
                context.set_source_color(*colors::PISTE);
                context.set_dash(&[9.5, 1.5], 0.0);
                draw();
                // TODO Road
            }
            (14.., _, "footway" | "pedestrian" | "platform") => {
                apply_highway_defaults(1.0);
                context.set_dash(&[4.0, 2.0], 0.0);
                draw();
                // TODO Road
            }
            (14.., _, "steps") => {
                // TODO <LinePatternSymbolizer file="images/steps.svg" />
            }
            (12.., _, _)
                if typ == "service" && service != "parking_aisle"
                    || ["escape", "corridor", "bus_guideway"].contains(&typ) =>
            {
                apply_highway_defaults(ke() * 1.2);
                draw();
                // TODO Road
            }
            (12.., _, _)
                if typ == "path"
                    && (row.get::<_, &str>("bicycle") != "designated"
                        || row.get::<_, &str>("foot") == "designated")
                    && (zoom > 12 || row.get("is_in_route")) =>
            {
                apply_highway_defaults(ke());
                context.set_dash(&[3.0, 3.0], 0.0);
                // TODO strokeOpacity="[trail_visibility]"
                draw();
                // TODO Road
            }
            (12.., _, _)
                if typ == "path"
                    && row.get::<_, &str>("bicycle") == "designated"
                    && row.get::<_, &str>("foot") == "designated"
                    && (zoom > 12 || row.get("is_in_route")) =>
            {
                apply_highway_defaults(ke());
                context.set_dash(&[4.0, 2.0], 0.0);
                context.set_source_color(*colors::CYCLEWAY);
                // TODO strokeOpacity="[trail_visibility]"
                draw();
                // TODO Road
            }
            (12.., _, _)
                if (typ == "cycleway"
                    || typ == "path"
                        && row.get::<_, &str>("bicycle") == "designated"
                        && row.get::<_, &str>("foot") != "designated")
                    && (zoom > 12 || row.get("is_in_route")) =>
            {
                apply_highway_defaults(ke());
                context.set_dash(&[6.0, 3.0], 0.0);
                context.set_source_color(*colors::CYCLEWAY);
                // TODO strokeOpacity="[trail_visibility]"
                draw();
                // TODO Road
            }
            (12.., _, "bridleway") if zoom > 12 || row.get("is_in_route") => {
                apply_highway_defaults(ke());
                context.set_dash(&[6.0, 3.0], 0.0);
                context.set_source_color(*colors::BRIDLEWAY);
                // TODO strokeOpacity="[trail_visibility]"
                draw();
                // TODO Road
            }
            (12.., _, "via_ferrata") if zoom > 12 || row.get("is_in_route") => {
                apply_highway_defaults(ke());
                context.set_dash(&[4.0, 4.0], 0.0);
                draw();
                // TODO Road
            }
            (12.., "highway", "track")
                if (zoom > 12
                    || row.get("is_in_route")
                    || row.get::<_, &str>("tracktype") == "grade1") =>
            {
                apply_highway_defaults(ke() * 1.2);
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
                // TODO strokeOpacity="[trail_visibility]"
                draw();
                // TODO Road
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
    }
}
