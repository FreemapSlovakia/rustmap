use crate::{
    colors::{self, Color, ContextExt},
    ctx::Ctx,
    draw::draw_mpoly,
    xyz::to_absolute_pixel_coords,
};
use cairo::{Content, Context, Extend, Matrix, RecordingSurface, Rectangle, SurfacePattern};
use postgis::ewkb::Geometry;
use postgres::Client;
use rsvg::Loader;

pub fn render(ctx: &Ctx, client: &mut Client) {
    let Ctx {
        bbox: (min_x, min_y, max_x, max_y),
        zoom,
        context,
        ..
    } = ctx;

    let query = &format!(
        "SELECT
                CASE WHEN type = 'wetland' AND tags->'wetland' IN ('bog', 'reedbed', 'marsh', 'swamp', 'wet_meadow', 'mangrove', 'fen') THEN tags->'wetland' ELSE type END AS type,
                geometry,
                position(type || ',' IN 'pedestrian,footway,pitch,library,baracks,parking,cemetery,place_of_worship,dam,weir,clearcut,wetland,scrub,orchard,vineyard,railway,landfill,scree,quarry,park,garden,allotments,village_green,grass,recreation_ground,fell,bare_rock,heath,meadow,wood,forest,golf_course,grassland,farm,zoo,farmyard,hospital,kindergarten,school,college,university,retail,commercial,industrial,residential,farmland,') AS z_order
            FROM osm_landusages{}
            WHERE geometry && ST_MakeEnvelope($1, $2, $3, $4, 3857)
            ORDER BY z_order DESC, osm_id",
        match zoom {
            ..=9 => "_gen0",
            10..=11 => "_gen1",
            12.. => "",
        }
    );

    for row in client.query(query, &[min_x, min_y, max_x, max_y]).unwrap() {
        let geom: Geometry = row.get("geometry");

        let colour_area = |color: Color| {
            context.set_source_color(color);
            draw_mpoly(ctx, &geom);
            context.fill().unwrap();
        };

        let pattern_area = |path: &str| {
            let handle = Loader::new().read_path(path).unwrap();

            let renderer = rsvg::CairoRenderer::new(&handle);

            let dim = renderer.intrinsic_size_in_pixels().unwrap();

            let tile = RecordingSurface::create(
                Content::ColorAlpha,
                Some(Rectangle::new(0.0, 0.0, dim.0, dim.1)),
            )
            .unwrap();

            {
                let context = Context::new(&tile).unwrap();

                renderer
                    .render_document(&context, &cairo::Rectangle::new(0.0, 0.0, dim.0, dim.1))
                    .unwrap();
            }

            let pattern = SurfacePattern::create(tile);

            let (x, y) = to_absolute_pixel_coords(*min_x, *min_y, *zoom as u8);

            let mut matrix = Matrix::identity();
            matrix.translate((x % dim.0).round(), (y % dim.1).round());
            pattern.set_matrix(matrix);

            pattern.set_extend(Extend::Repeat);

            context.set_source(&pattern).unwrap();

            draw_mpoly(ctx, &geom);

            context.fill().unwrap();
        };

        let typ: &str = row.get("type");

        match typ {
            "allotments" => {
                colour_area(*colors::ALLOTMENTS);
            }
            "cemetery" => {
                colour_area(*colors::GRASSY);
                pattern_area("images/grave.svg");
            }
            "clearcut" => {
                pattern_area("images/clearcut2.svg");
            }
            "bare_rock" => {
                pattern_area("images/bare_rock.svg");
            }
            "beach" => {
                colour_area(*colors::BEACH);
                pattern_area("images/sand.svg");
            }
            "brownfield" => {
                colour_area(*colors::BROWNFIELD);
            }
            "bog" => {
                colour_area(*colors::GRASSY);
                pattern_area("images/wetland.svg");
                pattern_area("images/bog.svg");
            }
            "college" => {
                colour_area(*colors::COLLEGE);
            }
            "commercial" => {
                colour_area(*colors::COMMERCIAL);
            }
            "dam" => {
                colour_area(*colors::DAM);
            }
            "farmland" => {
                colour_area(*colors::FARMLAND);
            }
            "farmyard" => {
                colour_area(*colors::FARMYARD);
            }
            "fell" => {
                colour_area(*colors::GRASSY);
            }
            "marsh" | "wet_meadow" | "fen" => {
                colour_area(*colors::GRASSY);
                pattern_area("images/wetland.svg");
                pattern_area("images/marsh.svg");
            }
            "footway" => {
                colour_area(*colors::NONE);
            }
            "forest" => {
                colour_area(*colors::FOREST);
            }
            "garages" => {
                colour_area(*colors::NONE);
            }
            "grass" => {
                colour_area(*colors::GRASSY);
            }
            "garden" => {
                colour_area(*colors::ORCHARD);

                context.set_source_rgba(0.0, 0.0, 0.0, 0.2);
                context.set_line_width(1.0);
                draw_mpoly(ctx, &geom);
                context.stroke().unwrap();
            }
            "grassland" => {
                colour_area(*colors::GRASSY);
            }
            "heath" => {
                colour_area(*colors::HEATH);
            }
            "hospital" => {
                colour_area(*colors::HOSPITAL);
            }
            "industrial" => {
                colour_area(*colors::INDUSTRIAL);
            }
            "landfill" => {
                colour_area(*colors::LANDFILL);
            }
            "living_street" => {
                colour_area(*colors::RESIDENTIAL);
            }
            "mangrove" => {
                colour_area(*colors::GRASSY);
                pattern_area("images/wetland.svg");
                pattern_area("images/mangrove.svg");
            }
            "meadow" => {
                colour_area(*colors::GRASSY);
            }
            "military" => {
                colour_area(*colors::NONE);
            }
            "orchard" => {
                colour_area(*colors::ORCHARD);
                pattern_area("images/orchard.svg");
            }
            "park" => {
                colour_area(*colors::GRASSY);
            }
            "parking" => {
                colour_area(*colors::PARKING);

                context.set_source_color(*colors::PARKING_STROKE);
                context.set_line_width(1.0);
                draw_mpoly(ctx, &geom);
                context.stroke().unwrap();
            }
            "pedestrian" => {
                colour_area(*colors::NONE);
            }
            "pitch" | "playground" | "golf_course" | "track" => {
                colour_area(*colors::PITCH);

                context.set_source_color(*colors::PITCH_STROKE);
                context.set_line_width(1.0);
                draw_mpoly(ctx, &geom);
                context.stroke().unwrap();
            }
            "plant_nursery" => {
                colour_area(*colors::SCRUB);
                pattern_area("images/plant_nursery.svg");
            }
            "quarry" => {
                colour_area(*colors::QUARRY);
                pattern_area("images/quarry.svg");
            }
            "railway" => {
                colour_area(*colors::NONE);
            }
            "reedbed" => {
                colour_area(*colors::GRASSY);
                pattern_area("images/wetland.svg");
                pattern_area("images/reedbed.svg");
            }
            "recreation_ground" => {
                colour_area(*colors::NONE);
            }
            "residential" => {
                colour_area(*colors::RESIDENTIAL);
            }
            "retail" => {
                colour_area(*colors::COMMERCIAL);
            }
            "silo" => {
                colour_area(*colors::SILO);

                context.set_source_color(*colors::SILO_STROKE);
                context.set_line_width(1.0);
                draw_mpoly(ctx, &geom);
                context.stroke().unwrap();
            }
            "school" => {
                colour_area(*colors::COLLEGE);
            }
            "scree" => {
                colour_area(*colors::SCREE);
                pattern_area("images/scree.svg");
            }
            "scrub" => {
                colour_area(*colors::SCRUB);
                pattern_area("images/scrub.svg");
            }
            "swamp" => {
                colour_area(*colors::GRASSY);
                pattern_area("images/wetland.svg");
                pattern_area("images/swamp.svg");
            }
            "university" => {
                colour_area(*colors::COLLEGE);
            }
            "village_green" => {
                colour_area(*colors::GRASSY);
            }
            "vineyard" => {
                colour_area(*colors::ORCHARD);
                pattern_area("images/grapes.svg");
            }
            "wastewater_plant" => {
                colour_area(*colors::INDUSTRIAL);
            }
            "weir" => {
                colour_area(*colors::DAM);
            }
            "wetland" => {
                pattern_area("images/wetland.svg");
            }
            "wood" => {
                colour_area(*colors::FOREST);
            }
            _ => (),
        }
    }
}
