use crate::{ctx::Ctx, draw::path_geom::path_geometry, projectable::TileProjectable};
use cairo::{Format, ImageSurface, Operator};
use geo::{BoundingRect, Geometry, Intersects, Rect, wkt};
use image::{GrayImage, imageops};

const BLUR_RADIUS_PX: f64 = 10.0;

pub fn render(ctx: &Ctx) {
    let _span = tracy_client::span!("blur_edges::render");

    let mask_polygon_merc = mask_polygon_web_mercator();

    if !tile_intersects_mask(&mask_polygon_merc, ctx) {
        clear_tile(ctx);
        return;
    }

    let width = (ctx.size.width as f64) as i32;
    let height = (ctx.size.height as f64) as i32;
    let blur_sigma_px = BLUR_RADIUS_PX;
    let pad = (blur_sigma_px * 3.0).ceil() as i32;

    if width <= 0 || height <= 0 {
        return;
    }

    let mask_geometry = mask_polygon_merc.project_to_tile(&ctx.tile_projector);

    let padded_w = width.saturating_add(pad * 2);
    let padded_h = height.saturating_add(pad * 2);

    let mut mask_surface = ImageSurface::create(Format::A8, padded_w, padded_h).unwrap();

    {
        let mask_ctx = cairo::Context::new(&mask_surface).unwrap();

        mask_ctx.translate(pad as f64, pad as f64);
        path_geometry(&mask_ctx, &mask_geometry);
        mask_ctx.set_source_rgba(1.0, 1.0, 1.0, 1.0);
        mask_ctx.fill().unwrap();
        mask_surface.flush();
    }

    let mask_width = mask_surface.width() as u32;
    let mask_height = mask_surface.height() as u32;
    let stride = mask_surface.stride() as usize;

    let alpha: Vec<u8> = {
        let data = mask_surface.data().unwrap();

        data.chunks(stride)
            .take(mask_height as usize)
            .flat_map(|row| row.iter().copied().take(mask_width as usize))
            .collect()
    };

    let gray =
        GrayImage::from_vec(mask_width, mask_height, alpha).expect("valid mask alpha buffer");

    let blurred = imageops::blur(&gray, blur_sigma_px as f32).into_raw();

    let mut blurred_rgba = vec![0u8; blurred.len() * 4];

    for (i, alpha) in blurred.iter().enumerate() {
        let idx = i * 4;
        blurred_rgba[idx + 3] = *alpha;
    }

    let blurred_surface = ImageSurface::create_for_data(
        blurred_rgba,
        Format::ARgb32,
        padded_w,
        padded_h,
        (mask_width * 4) as i32,
    )
    .unwrap();

    let context = ctx.context;

    context.save().unwrap();
    context.identity_matrix();
    context.set_operator(Operator::DestIn);
    context
        .set_source_surface(&blurred_surface, -(pad as f64), -(pad as f64))
        .unwrap();
    context.paint().unwrap();
    context.set_operator(Operator::DestOver);
    context.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    context.paint().unwrap();

    context.restore().unwrap();
}

fn mask_polygon_web_mercator() -> Geometry {
    wkt! {
      MULTIPOLYGON (((2309150.73361148 4866305.98301504,3339266.22636634 4866305.98301504,3339266.22636634 6799780.91284591,556939.142958447 6799780.91284591,556939.142958447 6473911.15913782,418260.972791162 6555241.99172229,261717.938863121 6702001.08602965,78269.0709786979 6550350.02191064,-339994.347797787 6462294.5653274,-675094.279799999 6188344.25595305,-233897.612300944 5302284.08383483,195066.073191387 5192827.17657414,402842.093935532 5192827.17657414,859763.699369764 4640033.42843813,1257541.68359969 4475846.93401446,1287810.44105232 4364860.67469239,1692620.33135428 4367307.27109437,2246025.63913134 4766307.33575086,2309150.73361148 4866305.98301504)))
    }.into()
}

fn tile_intersects_mask(mask: &Geometry, ctx: &Ctx) -> bool {
    let tile_bbox = &ctx.bbox;
    let Some(mut bbox) = mask.bounding_rect() else {
        return false;
    };

    let blur_radius_m = BLUR_RADIUS_PX as f64 * ctx.meters_per_pixel() * 3.0;

    bbox = Rect::new(
        (bbox.min().x - blur_radius_m, bbox.min().y - blur_radius_m),
        (bbox.max().x + blur_radius_m, bbox.max().y + blur_radius_m),
    );

    bbox.intersects(tile_bbox)
}

fn clear_tile(ctx: &Ctx) {
    let context = ctx.context;
    context.save().unwrap();
    context.identity_matrix();
    context.set_operator(Operator::Clear);
    context.paint().unwrap();
    context.restore().unwrap();
}
