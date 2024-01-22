use bitflags::bitflags;
use postgis::ewkb::MultiLineString;
use postgres::Client;

use crate::{
    ctx::Ctx,
    draw::draw_line_off,
};

const COLOR_SQL: &str = r#"
  CASE
    WHEN "osmc:symbol" LIKE 'red:%' THEN 0
    WHEN "osmc:symbol" LIKE 'blue:%' THEN 1
    WHEN "osmc:symbol" LIKE 'green:%' THEN 2
    WHEN "osmc:symbol" LIKE 'yellow:%' THEN 3
    WHEN "osmc:symbol" LIKE 'black:%' THEN 4
    WHEN "osmc:symbol" LIKE 'white:%' THEN 5
    WHEN "osmc:symbol" LIKE 'orange:%' THEN 6
    WHEN "osmc:symbol" LIKE 'violet:%' THEN 7
    WHEN "osmc:symbol" LIKE 'purple:%' THEN 7
    WHEN colour = 'red' THEN 0
    WHEN colour = 'blue' THEN 1
    WHEN colour = 'green' THEN 2
    WHEN colour = 'yellow' THEN 3
    WHEN colour = 'black' THEN 4
    WHEN colour = 'white' THEN 5
    WHEN colour = 'orange' THEN 6
    WHEN colour = 'violet' THEN 7
    WHEN colour = 'purple' THEN 7
    ELSE 8
  END
"#;

const COLORS: [(&str, (f64, f64, f64)); 9] = [
    (
        "none",
        (
            0xa0 as f64 / 255.0,
            0xa0 as f64 / 255.0,
            0xa0 as f64 / 255.0,
        ),
    ),
    (
        "purple",
        (
            0xc0 as f64 / 255.0,
            0x00 as f64 / 255.0,
            0xc0 as f64 / 255.0,
        ),
    ),
    (
        "orange",
        (
            0xff as f64 / 255.0,
            0x80 as f64 / 255.0,
            0x00 as f64 / 255.0,
        ),
    ),
    (
        "white",
        (
            0xff as f64 / 255.0,
            0xff as f64 / 255.0,
            0xff as f64 / 255.0,
        ),
    ),
    (
        "black",
        (
            0x00 as f64 / 255.0,
            0x00 as f64 / 255.0,
            0x00 as f64 / 255.0,
        ),
    ),
    (
        "yellow",
        (
            0xf0 as f64 / 255.0,
            0xf0 as f64 / 255.0,
            0x00 as f64 / 255.0,
        ),
    ),
    (
        "green",
        (
            0x00 as f64 / 255.0,
            0xa0 as f64 / 255.0,
            0x00 as f64 / 255.0,
        ),
    ),
    (
        "blue",
        (
            0x50 as f64 / 255.0,
            0x50 as f64 / 255.0,
            0xff as f64 / 255.0,
        ),
    ),
    (
        "red",
        (
            0xff as f64 / 255.0,
            0x30 as f64 / 255.0,
            0x30 as f64 / 255.0,
        ),
    ),
];

bitflags! {
  pub struct RouteTypes: u32 {
      const HIKING = 0b00000001;
      const HORSE = 0b00000010;
      const BICYCLE = 0b00000100;
      const SKI = 0b00001000;
  }
}

fn format_vec(vec: &Vec<&str>) -> String {
    if vec.is_empty() {
        "'_x_'".to_string()
    } else {
        vec.iter()
            .map(|&item| format!("'{}'", item))
            .collect::<Vec<String>>()
            .join(",")
    }
}

fn get_routes_query(
    route_types: &RouteTypes,
    include_networks: Option<Vec<&str>>,
    gen_suffix: &str,
) -> String {
    let mut lefts = Vec::<&str>::new();

    let mut rights = Vec::<&str>::new();

    if route_types.contains(RouteTypes::HIKING) {
        lefts.extend_from_slice(&["hiking", "foot", "running"]);
    }

    if route_types.contains(RouteTypes::HORSE) {
        lefts.push("horse");
    }

    if route_types.contains(RouteTypes::BICYCLE) {
        rights.extend_from_slice(&["bicycle", "mtb"]);
    }

    if route_types.contains(RouteTypes::SKI) {
        rights.extend_from_slice(&["ski", "piste"]);
    }

    let lefts_in = format_vec(&lefts);

    let rights_in = format_vec(&rights);

    let cond = match include_networks {
        None => String::from(""),
        Some(networks) => {
            let mut result = String::from("network IN (");

            for (i, &network) in networks.iter().enumerate() {
                if i > 0 {
                    result.push(',');
                }
                result.push('\'');
                result.push_str(network);
                result.push('\'');
            }

            result
        }
    };

    return format!(r#"
SELECT
  ST_Multi(ST_LineMerge(ST_Collect(geometry))) AS geometry,
  idx(arr1, 0) AS h_red,
  idx(arr1, 1) AS h_blue,
  idx(arr1, 2) AS h_green,
  idx(arr1, 3) AS h_yellow,
  idx(arr1, 4) AS h_black,
  idx(arr1, 5) AS h_white,
  idx(arr1, 6) AS h_orange,
  idx(arr1, 7) AS h_purple,
  idx(arr1, 8) AS h_none,
  idx(arr1, 10) AS h_red_loc,
  idx(arr1, 11) AS h_blue_loc,
  idx(arr1, 12) AS h_green_loc,
  idx(arr1, 13) AS h_yellow_loc,
  idx(arr1, 14) AS h_black_loc,
  idx(arr1, 15) AS h_white_loc,
  idx(arr1, 16) AS h_orange_loc,
  idx(arr1, 17) AS h_purple_loc,
  idx(arr1, 18) AS h_none_loc,
  idx(arr2, 20) AS b_red,
  idx(arr2, 21) AS b_blue,
  idx(arr2, 22) AS b_green,
  idx(arr2, 23) AS b_yellow,
  idx(arr2, 24) AS b_black,
  idx(arr2, 25) AS b_white,
  idx(arr2, 26) AS b_orange,
  idx(arr2, 27) AS b_purple,
  idx(arr2, 28) AS b_none,
  idx(arr2, 30) AS s_red,
  idx(arr2, 31) AS s_blue,
  idx(arr2, 32) AS s_green,
  idx(arr2, 33) AS s_yellow,
  idx(arr2, 34) AS s_black,
  idx(arr2, 35) AS s_white,
  idx(arr2, 36) AS s_orange,
  idx(arr2, 37) AS s_purple,
  idx(arr2, 38) AS s_none,
  idx(arr1, 40) AS r_red,
  idx(arr1, 41) AS r_blue,
  idx(arr1, 42) AS r_green,
  idx(arr1, 43) AS r_yellow,
  idx(arr1, 44) AS r_black,
  idx(arr1, 45) AS r_white,
  idx(arr1, 46) AS r_orange,
  idx(arr1, 47) AS r_purple,
  idx(arr1, 48) AS r_none,
  refs1,
  refs2,
  icount(arr1 - array[1000, 1010, 1020, 1030, 1040]) AS off1,
  icount(arr2 - array[1000, 1010, 1020, 1030, 1040]) AS off2
FROM (
  SELECT
    array_to_string(
      array(
        SELECT distinct itm FROM unnest(
          array_agg(
            CASE
              WHEN
                osm_routes.type IN ({lefts_in})
              THEN
                CASE
                  WHEN name <> '' AND ref <> ''
                  THEN name || ' (' || ref || ')'
                  ELSE COALESCE(NULLIF(name, ''), NULLIF(ref, '')) END
              ELSE
                null
              END
          )
        ) AS itm ORDER BY itm
      ),
      ', '
    ) AS refs1,
    array_to_string(
      array(
        SELECT distinct itm FROM unnest(
          array_agg(
            CASE
              WHEN
                osm_routes.type IN ({rights_in})
              THEN
                CASE
                  WHEN name <> '' AND ref <> ''
                  THEN name || ' (' || ref || ')'
                  ELSE COALESCE(NULLIF(name, ''), NULLIF(ref, '')) END
              ELSE
                null
              END
          )
        ) AS itm ORDER BY itm
      ),
      ', '
    ) AS refs2,
    first(geometry) AS geometry,
    uniq(sort(array_agg(
      CASE
        WHEN osm_routes.type IN ({lefts_in}) THEN
          CASE
            WHEN {bool_horse} AND osm_routes.type = 'horse' THEN 40
            WHEN {bool_hiking} AND osm_routes.type IN ('hiking', 'foot', 'running') THEN (CASE WHEN network IN ('iwn', 'nwn', 'rwn') THEN 0 ELSE 10 END)
            ELSE 1000
          END +
          {color_sql}
        ELSE 1000
      END
    ))) AS arr1,
    uniq(sort(array_agg(
      CASE
        WHEN osm_routes.type IN ({rights_in}) THEN
          CASE
            WHEN {bool_bicycle} AND osm_routes.type IN ('bicycle', 'mtb') THEN 20
            WHEN {bool_ski} AND osm_routes.type IN ('ski', 'piste') THEN 30
            ELSE 1000
          END +
          {color_sql}
        ELSE
          1000
        END
    ))) AS arr2
  FROM osm_route_members{gen_suffix} JOIN osm_routes ON (osm_route_members{gen_suffix}.osm_id = osm_routes.osm_id AND state <> 'proposed')
  WHERE {where}geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
  GROUP BY member
) AS aaa
GROUP BY
  h_red, h_blue, h_green, h_yellow, h_black, h_white, h_orange, h_purple, h_none,
  h_red_loc, h_blue_loc, h_green_loc, h_yellow_loc, h_black_loc, h_white_loc, h_orange_loc, h_purple_loc, h_none_loc,
  b_red, b_blue, b_green, b_yellow, b_black, b_white, b_orange, b_purple, b_none,
  s_red, s_blue, s_green, s_yellow, s_black, s_white, s_orange, s_purple, s_none,
  r_red, r_blue, r_green, r_yellow, r_black, r_white, r_orange, r_purple, r_none,
  off1, off2, refs1, refs2"#,
        lefts_in = lefts_in,
        rights_in = rights_in,
        bool_horse = route_types.contains(RouteTypes::HORSE),
        bool_hiking = route_types.contains(RouteTypes::HIKING),
        bool_bicycle = route_types.contains(RouteTypes::BICYCLE),
        bool_ski = route_types.contains(RouteTypes::SKI),
        gen_suffix = gen_suffix,
        color_sql = COLOR_SQL,
        where = cond
    );
}

pub fn render(ctx: &Ctx, client: &mut Client, route_types: &RouteTypes) {
    let Ctx {
        context,
        bbox: (min_x, min_y, max_x, max_y),
        ..
    } = ctx;

    let zoom = ctx.zoom;

    let query = match zoom {
        9 => get_routes_query(route_types, Some(vec!["iwn", "icn"]), "_gen0"),
        10 => get_routes_query(route_types, Some(vec!["iwn", "nwn", "icn", "ncn"]), "_gen1"),
        11 => get_routes_query(
            route_types,
            Some(vec!["iwn", "nwn", "rwn", "icn", "ncn", "rcn"]),
            "_gen1",
        ),
        12..=13 => get_routes_query(route_types, None, ""),
        14.. => get_routes_query(route_types, None, ""),
        _ => return,
    };

    let rows = &client.query(&query, &[min_x, min_y, max_x, max_y]).unwrap();

    for row in rows {
        let geom: MultiLineString = row.get("geometry");

        let (zo, wf) = match zoom {
            ..=11 => (1.0, 1.5),
            12 => (2.0, 1.5),
            13.. => (3.0, 2.0),
        }; // offset from highway

        let df = 1.25;

        for color in COLORS.iter() {
            if route_types.contains(RouteTypes::HORSE) {
                let off = row.get::<_, i32>(&format!("r_{}", color.0)[..]);

                if off > 0 {
                    let offset = (zo + (off as f64 - 1.0) * wf * df) + 0.5;

                    // <LinePatternSymbolizer
                    //   file={path.resolve(tmpdir(), `horse-${color}.svg`)}
                    //   offset={offset}
                    //   transform={`scale(${wf / 2})`}
                    // />
                }
            }

            if route_types.contains(RouteTypes::SKI) {
                let off = row.get::<_, i32>(&format!("s_{}", color.0)[..]);

                if off > 0 {
                    let offset = -(zo + (off as f64 - 1.0) * wf * 2.0) - 1.0;

                    // <LinePatternSymbolizer
                    //   file={path.resolve(tmpdir(), `ski-${color}.svg`)}
                    //   offset={offset}
                    //   transform={`scale(${wf / 2})`}
                    // />
                }
            }

            if route_types.contains(RouteTypes::BICYCLE) {
                let off = row.get::<_, i32>(&format!("b_{}", color.0)[..]);

                if off > 0 {
                    let offset = -(zo + (off as f64 - 1.0) * wf * 2.0) - 1.0;

                    for part in geom.lines.iter() {
                        draw_line_off(ctx, part.points.iter(), offset);
                    }

                    context.set_line_width(wf * 2.0);
                    context.set_line_join(cairo::LineJoin::Round);
                    context.set_line_cap(cairo::LineCap::Round);
                    context.set_source_rgb(color.1 .0, color.1 .1, color.1 .2);
                    context.set_dash(&[0.001, wf * 3.0], 0.0);

                    context.stroke().unwrap();
                }
            }

            if route_types.contains(RouteTypes::HIKING) {
                {
                    let off = row.get::<_, i32>(&format!("h_{}", color.0)[..]);

                    if off > 0 {
                        let offset = (zo + (off as f64 - 1.0) * wf * df) + 0.5;

                        for part in geom.lines.iter() {
                            draw_line_off(ctx, part.points.iter(), offset);
                        }
                        context.set_line_width(wf);
                        context.set_line_join(cairo::LineJoin::Round);
                        context.set_line_cap(cairo::LineCap::Butt);
                        context.set_source_rgb(color.1 .0, color.1 .1, color.1 .2);
                        context.set_dash(&[], 0.0);

                        context.stroke().unwrap();
                    }
                }

                {
                    let off = row.get::<_, i32>(&format!("h_{}_loc", color.0)[..]);

                    if off > 0 {
                        let offset = (zo + (off as f64 - 1.0) * wf * df) + 0.5;

                        for part in geom.lines.iter() {
                            draw_line_off(ctx, part.points.iter(), offset);
                        }

                        context.set_line_width(wf);
                        context.set_line_join(cairo::LineJoin::Round);
                        context.set_line_cap(cairo::LineCap::Butt);
                        context.set_source_rgb(color.1 .0, color.1 .1, color.1 .2);
                        context.set_dash(&[wf * 3.0, wf], 0.0);

                        context.stroke().unwrap();
                    }
                }
            }
        }
    }
}
