use crate::{
    ctx::Ctx, layer_render_error::LayerRenderResult,
    layers::hillshading_datasets::HillshadingDatasets,
};
use cairo::{Format, ImageSurface};
use gdal::Dataset;

fn read_rgba_from_gdal(
    dataset: &Dataset,
    ctx: &Ctx,
    gt_x_off: f64,
    gt_x_width: f64,
    gt_y_off: f64,
    gt_y_width: f64,
    raster_scale: f64,
) -> (ImageSurface, bool) {
    let bbox = ctx.bbox;
    let size = ctx.size;

    let min = bbox.min();
    let max = bbox.max();

    // Convert geographic coordinates (min_x, min_y, max_x, max_y) to pixel coordinates
    let pixel_min_x_f = (min.x - gt_x_off) / gt_x_width;
    let pixel_max_x_f = (max.x - gt_x_off) / gt_x_width;

    let pixel_min_x = pixel_min_x_f.floor() as isize;
    let pixel_max_x = pixel_max_x_f.ceil() as isize;

    let (pixel_min_y_f, pixel_max_y_f) = {
        let pixel_y0 = (min.y - gt_y_off) / gt_y_width;
        let pixel_y1 = (max.y - gt_y_off) / gt_y_width;

        (pixel_y0.min(pixel_y1), pixel_y0.max(pixel_y1))
    };

    let pixel_min_y = pixel_min_y_f.floor() as isize;
    let pixel_max_y = pixel_max_y_f.ceil() as isize;

    let window_width_px = (pixel_max_x - pixel_min_x) as usize;
    let window_height_px = (pixel_max_y - pixel_min_y) as usize;

    let scaled_width_px = (size.width as f64 * raster_scale) as usize;
    let scaled_height_px = (size.height as f64 * raster_scale) as usize;

    let scale_x = scaled_width_px as f64 / (pixel_max_x_f - pixel_min_x_f).abs().max(1e-6);
    let scale_y = scaled_height_px as f64 / (pixel_max_y_f - pixel_min_y_f).abs().max(1e-6);

    let buffered_w = (scale_x * window_width_px as f64).ceil().max(1.0) as usize;
    let buffered_h = (scale_y * window_height_px as f64).ceil().max(1.0) as usize;

    let mut rgba_data = vec![0u8; buffered_w * buffered_h * 4];

    let (raster_width, raster_height) = dataset.raster_size();

    // Adjust the window to fit within the raster bounds
    let clamped_window_x = pixel_min_x.max(0).min(raster_width as isize);
    let clamped_window_y = pixel_min_y.max(0).min(raster_height as isize);

    let clamped_source_width = ((pixel_min_x + window_width_px as isize).min(raster_width as isize)
        - clamped_window_x)
        .max(0) as usize;

    let clamped_source_height =
        ((pixel_min_y + window_height_px as isize).min(raster_height as isize) - clamped_window_y)
            .max(0) as usize;

    if clamped_source_width == 0 || clamped_source_height == 0 {
        return (
            ImageSurface::create_for_data(
                vec![0u8; scaled_width_px * scaled_height_px * 4],
                Format::ARgb32,
                (size.width as f64 * raster_scale) as i32,
                (size.height as f64 * raster_scale) as i32,
                (size.width as f64 * raster_scale) as i32 * 4,
            )
            .unwrap(),
            false,
        );
    }

    let resampled_width = (buffered_w as f64
        * (clamped_source_width as f64 / window_width_px as f64))
        .ceil() as usize;

    let resampled_height = (buffered_h as f64
        * (clamped_source_height as f64 / window_height_px as f64))
        .ceil() as usize;

    let offset_x = (((clamped_window_x - pixel_min_x) as f64 / window_width_px as f64)
        * buffered_w as f64)
        .floor()
        .max(0.0) as usize;

    let offset_y = (((clamped_window_y - pixel_min_y) as f64 / window_height_px as f64)
        * buffered_h as f64)
        .floor()
        .max(0.0) as usize;

    let copy_width = resampled_width.min(buffered_w.saturating_sub(offset_x));
    let copy_height = resampled_height.min(buffered_h.saturating_sub(offset_y));

    let mut band_buffer = vec![0u8; resampled_height * resampled_width];

    let mut has_data = false;

    let band_count = dataset.raster_count();

    for band_index in 0..band_count {
        let band = dataset.rasterband(band_index + 1).unwrap();

        let no_data = band.no_data_value();

        if clamped_source_width > 0
            && clamped_source_height > 0
            && resampled_width > 0
            && resampled_height > 0
        {
            band.read_into_slice::<u8>(
                (clamped_window_x, clamped_window_y),
                (clamped_source_width, clamped_source_height),
                (resampled_width, resampled_height), // Resampled size
                &mut band_buffer,
                Some(gdal::raster::ResampleAlg::Lanczos),
            )
            .unwrap();
        }

        for y in 0..copy_height {
            for x in 0..copy_width {
                let data_index = y * resampled_width + x;

                let rgba_index = ((y + offset_y) * buffered_w + (x + offset_x)) * 4;

                let value = band_buffer[data_index];
                let is_no_data = no_data.is_some_and(|nd| (nd as u8) == value);

                if !is_no_data {
                    has_data = true;
                }

                match (band_count, band_index) {
                    (1, _) => {
                        rgba_data[rgba_index] = value;
                        rgba_data[rgba_index + 1] = value;
                        rgba_data[rgba_index + 2] = value;
                        rgba_data[rgba_index + 3] = no_data
                            .map_or(255u8, |nd| if (nd as u8) == value { 0u8 } else { 255u8 });
                    }
                    (2, 0) => {
                        rgba_data[rgba_index] = value;
                        rgba_data[rgba_index + 1] = value;
                        rgba_data[rgba_index + 2] = value;
                    }
                    (2, _) => {
                        // alpha
                        rgba_data[rgba_index + 3] = value;
                    }
                    (3, _) => {
                        if band_index == 0 {
                            rgba_data[rgba_index + 3] = 255;
                        }
                        rgba_data[rgba_index + band_index] = value;
                    }
                    (4, _) => {
                        rgba_data[rgba_index + band_index] = value;
                    }
                    _ => panic!("unsupported band count"),
                }
            }
        }
    }

    let frac_x = pixel_min_x_f - pixel_min_x as f64;
    let frac_y = pixel_min_y_f - pixel_min_y as f64;

    let crop_x_base = offset_x + (frac_x * scale_x).round().max(0.0) as usize;
    let crop_y_base = offset_y + (frac_y * scale_y).round().max(0.0) as usize;

    // If rounding pushed the origin too far, clamp so we still copy a full tile when possible.
    let crop_x = crop_x_base.min(buffered_w.saturating_sub(scaled_width_px));
    let crop_y = crop_y_base.min(buffered_h.saturating_sub(scaled_height_px));

    let crop_w = scaled_width_px.min(buffered_w.saturating_sub(crop_x));
    let crop_h = scaled_height_px.min(buffered_h.saturating_sub(crop_y));

    let mut final_rgba_data = vec![0u8; scaled_width_px * scaled_height_px * 4];

    if crop_w > 0 && crop_h > 0 && crop_x < buffered_w && crop_y < buffered_h {
        for y in 0..crop_h {
            let src_offset = ((y + crop_y) * buffered_w + crop_x) * 4;
            let dst_offset = y * scaled_width_px * 4;

            // Guard against any edge rounding that would push past the buffer.
            let max_copy = ((buffered_w - crop_x) * 4).min(crop_w * 4);
            let src_end = (src_offset + max_copy).min(rgba_data.len());
            let dst_end = dst_offset + (src_end - src_offset);

            if src_end > src_offset && dst_end > dst_offset {
                final_rgba_data[dst_offset..dst_end]
                    .copy_from_slice(&rgba_data[src_offset..src_end]);
            }
        }
    }

    for i in (0..final_rgba_data.len()).step_by(4) {
        let alpha = final_rgba_data[i + 3] as f32 / 255.0;

        let r = (final_rgba_data[i] as f32 * alpha) as u8;
        let g = (final_rgba_data[i + 1] as f32 * alpha) as u8;
        let b = (final_rgba_data[i + 2] as f32 * alpha) as u8;

        final_rgba_data[i] = b;
        final_rgba_data[i + 1] = g;
        final_rgba_data[i + 2] = r;
    }

    let surface = ImageSurface::create_for_data(
        final_rgba_data,
        Format::ARgb32,
        (size.width as f64 * raster_scale) as i32,
        (size.height as f64 * raster_scale) as i32,
        (size.width as f64 * raster_scale) as i32 * 4,
    )
    .unwrap();

    (surface, has_data)
}

pub fn render(
    ctx: &Ctx,
    country: &str,
    alpha: f64,
    shading_data: &mut HillshadingDatasets,
    raster_scale: f64,
) -> LayerRenderResult {
    let (surface, used_data) = {
        let hillshading_dataset = shading_data
            .get(country)
            .unwrap_or_else(|| panic!("no such dataset {country}"));

        let [gt_x_off, gt_x_width, _, gt_y_off, _, gt_y_width] =
            hillshading_dataset.geo_transform().unwrap();

        read_rgba_from_gdal(
            hillshading_dataset,
            ctx,
            gt_x_off,
            gt_x_width,
            gt_y_off,
            gt_y_width,
            raster_scale,
        )
    };

    if used_data {
        shading_data.record_use(country);
    }

    let context = ctx.context;

    context.save()?;

    if raster_scale != 1.0 {
        context.scale(1.0 / raster_scale, 1.0 / raster_scale);
    }

    context.set_source_surface(surface, 0.0, 0.0)?;

    context.paint_with_alpha(alpha)?;

    context.restore()?;

    Ok(())
}
