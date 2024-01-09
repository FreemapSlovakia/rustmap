use crate::{colors, ctx::Ctx, draw::draw_mpoly};
use postgis::ewkb::Geometry;
use postgres::Client;
use std::collections::HashMap;

lazy_static! {
    static ref LANDUSE_COLORS: HashMap<&'static str, (f64, f64, f64)> = HashMap::from([
        ("allotments", *colors::ALLOTMENTS),
        ("cemetery", *colors::GRASSY),
        // ("bare_rock", ...),
        ("beach", *colors::BEACH),
        ("brownfield", *colors::BROWNFIELD),
        // ("cemetery", ...),
        ("college", *colors::COLLEGE),
        ("commercial", *colors::COMMERCIAL),
        ("dam", *colors::DAM),
        ("farmland", *colors::FARMLAND),
        ("farmyard", *colors::FARMYARD),
        ("fell", *colors::GRASSY),
        ("footway", *colors::NONE),
        ("forest", *colors::FOREST),
        ("garages", *colors::NONE),
        ("grass", *colors::GRASSY),
        ("garden", *colors::ORCHARD),
        ("grassland", *colors::GRASSY),
        ("heath", *colors::HEATH),
        ("hospital", *colors::HOSPITAL),
        ("industrial", *colors::INDUSTRIAL),
        ("landfill", *colors::LANDFILL),
        ("living_street", *colors::RESIDENTIAL),
        ("meadow", *colors::GRASSY),
        ("military", *colors::NONE),
        ("orchard", *colors::ORCHARD),
        ("park", *colors::GRASSY),
        ("parking", *colors::PARKING),
        ("pedestrian", *colors::NONE),
        ("pitch", *colors::PITCH),
        ("plant_nursery", *colors::SCRUB),
        ("quarry", *colors::QUARRY),
        ("railway", *colors::NONE),
        ("recreation_ground", *colors::NONE),
        ("residential", *colors::RESIDENTIAL),
        ("retail", *colors::COMMERCIAL),
        ("school", *colors::COLLEGE),
        ("scree", *colors::SCREE),
        ("scrub", *colors::SCRUB),
        ("university", *colors::COLLEGE),
        ("village_green", *colors::GRASSY),
        ("vineyard", *colors::ORCHARD),
        ("wastewater_plant", *colors::INDUSTRIAL),
        // ("water", *colors::WATER),
        ("weir", *colors::DAM),
        ("wood", *colors::FOREST),
    ]);
}

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        context,
        ..
    } = ctx;

    for row in client.query(
        "SELECT type, geometry FROM osm_landusages WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)",
        &[&min_x, &min_y, &max_x, &max_y]
    ).unwrap() {
        let geom: Geometry = row.get("geometry");

        let t: &str = row.get("type");

        let default_color = (1.0, 0.5, 0.5);

        let (r, g, b) = LANDUSE_COLORS.get(t).unwrap_or(&default_color);

        context.set_source_rgb(*r, *g, *b);

        draw_mpoly(geom, &ctx);
  }
}
