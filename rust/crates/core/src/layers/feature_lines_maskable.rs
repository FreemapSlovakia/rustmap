use crate::{
    SvgRepo,
    colors::{self, ContextExt},
    ctx::Ctx,
    draw::{line_pattern::draw_line_pattern, path_geom::path_line_string},
    layer_render_error::LayerRenderResult,
    layers::{hillshading, hillshading_datasets::HillshadingDatasets},
    projectable::{TileProjectable, geometry_line_string},
};
use postgres::Client;

pub fn render(
    ctx: &Ctx,
    client: &mut Client,
    svg_cache: &mut SvgRepo,
    hillshading_datasets: &mut Option<HillshadingDatasets>,
    shading: bool,
    hillshade_scale: f64,
) -> LayerRenderResult {
    let _span = tracy_client::span!("feature_lines_maskable::render");

    let sql = "
        SELECT geometry, type
        FROM osm_feature_lines
        WHERE
            type NOT IN ('cutline', 'valley', 'ridge') AND
            geometry && ST_Expand(ST_MakeEnvelope($1, $2, $3, $4, 3857), $5)";

    let rows = client.query(sql, &ctx.bbox_query_params(Some(8.0)).as_params())?;

    let context = ctx.context;

    context.push_group();

    for row in rows {
        let geom = geometry_line_string(&row).project_to_tile(&ctx.tile_projector);

        let zoom = ctx.zoom;

        match row.get("type") {
            "earth_bank" => {
                if zoom >= 14 {
                    draw_line_pattern(ctx, &geom, 0.8, svg_cache.get("earth_bank.svg")?)?;
                }
            }
            "dyke" => {
                if zoom >= 14 {
                    draw_line_pattern(ctx, &geom, 0.8, svg_cache.get("dyke.svg")?)?;
                }
            }
            "embankment" => {
                if zoom >= 14 {
                    draw_line_pattern(ctx, &geom, 0.8, svg_cache.get("embankment-half.svg")?)?;
                }
            }
            "gully" => {
                if zoom >= 14 {
                    draw_line_pattern(ctx, &geom, 0.8, svg_cache.get("gully.svg")?)?;
                }
            }
            "cliff" => {
                if zoom >= 13 {
                    draw_line_pattern(ctx, &geom, 0.8, svg_cache.get("cliff.svg")?)?;

                    context.set_source_color(colors::AREA_LABEL);
                    context.set_line_width(1.0);
                    path_line_string(context, &geom);
                    context.stroke()?;
                }
            }
            _ => {
                //
            }
        }
    }

    context.push_group();

    if shading && let Some(hillshading_datasets) = hillshading_datasets {
        for cc in [
            "pl", "sk", "cz", "at", /*"ch", "it" (CH, IT are not so detailed) */
        ] {
            hillshading::render(
                ctx,
                &format!("{cc}-mask"),
                1.0,
                hillshading_datasets,
                hillshade_scale,
            )?;
        }
    }

    context.pop_group_to_source()?;

    context.set_operator(cairo::Operator::DestOut);

    context.paint()?;

    context.pop_group_to_source()?;

    context.paint()?;

    Ok(())
}
