use crate::{
    collision::Collision,
    colors::{self, Color, ContextExt},
    draw::create_pango_layout::{FontAndLayoutOptions, create_pango_layout},
};
use cairo::Context;
use geo::{Coord, Distance, Euclidean, InterpolatePoint, LineString, Rect};
use pangocairo::{
    functions::glyph_string_path,
    pango::{Font, GlyphItem, GlyphString, Layout, SCALE},
};
use std::f64::consts::{PI, TAU};

#[derive(Copy, Clone, Debug)]
pub struct TextOnLineOptions {
    pub upright: Upright,
    pub align: Align,
    pub repeat_distance: Option<f64>,
    pub spacing: f64,
    pub alpha: f64,
    pub color: Color,
    pub halo_color: Color,
    pub halo_opacity: f64,
    pub halo_width: f64,
    pub max_curvature_degrees: f64,
    pub concave_spacing_factor: f64,
    pub flo: FontAndLayoutOptions,
}

impl Default for TextOnLineOptions {
    fn default() -> Self {
        TextOnLineOptions {
            upright: Upright::Auto,
            align: Align::Center,
            repeat_distance: None,
            spacing: 0.0,
            alpha: 1.0,
            color: colors::BLACK,
            halo_color: colors::WHITE,
            halo_opacity: 0.75,
            halo_width: 1.5,
            max_curvature_degrees: 60.0,
            concave_spacing_factor: 1.0,
            flo: FontAndLayoutOptions::default(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Upright {
    Left,
    Right,
    Auto,
}

#[derive(Copy, Clone, Debug)]
pub enum Align {
    Left,
    Center,
    Right,
    Justify,
}

// TODO check geo crate
fn normalize(v: Coord) -> Coord {
    let len = v.x.hypot(v.y);
    if len == 0.0 {
        Coord { x: 0.0, y: 0.0 }
    } else {
        Coord {
            x: v.x / len,
            y: v.y / len,
        }
    }
}

// TODO check geo crate
fn angle_between(a: Coord, b: Coord) -> f64 {
    let dot = a.x * b.x + a.y * b.y;
    let det = a.x * b.y - a.y * b.x;
    det.atan2(dot).abs().to_degrees()
}

// TODO check geo crate
fn normalize_angle(a: f64) -> f64 {
    if a > PI {
        a - TAU
    } else if a <= -PI {
        a + TAU
    } else {
        a
    }
}

fn adjust_upright_angle(angle: f64, upright: Upright) -> f64 {
    let a = normalize_angle(angle);

    match upright {
        Upright::Left => normalize_angle(a + PI),
        Upright::Right => a,
        Upright::Auto => {
            if a.abs() > PI / 2.0 {
                normalize_angle(a + PI)
            } else {
                a
            }
        }
    }
}

fn weighted_tangent_for_span(
    pts: &[Coord],
    cum: &[f64],
    span_start: f64,
    span_end: f64,
) -> Option<Coord> {
    if pts.len() < 2 {
        return None;
    }

    let mut accum = Coord { x: 0.0, y: 0.0 };
    let mut total = 0.0;

    for i in 0..pts.len() - 1 {
        let seg_start = cum[i];
        let seg_end = cum[i + 1];

        let overlap_start = span_start.max(seg_start);
        let overlap_end = span_end.min(seg_end);

        if overlap_end <= overlap_start {
            continue;
        }

        let weight = overlap_end - overlap_start;
        let tangent = normalize(Coord {
            x: pts[i + 1].x - pts[i].x,
            y: pts[i + 1].y - pts[i].y,
        });

        accum.x += tangent.x * weight;
        accum.y += tangent.y * weight;
        total += weight;
    }

    if total == 0.0 {
        None
    } else {
        Some(normalize(accum))
    }
}

fn tangents_for_span(pts: &[Coord], cum: &[f64], span_start: f64, span_end: f64) -> Vec<Coord> {
    let mut result = Vec::new();

    for i in 0..pts.len() - 1 {
        let seg_start = cum[i];
        let seg_end = cum[i + 1];

        let overlap_start = span_start.max(seg_start);
        let overlap_end = span_end.min(seg_end);

        if overlap_end <= overlap_start {
            continue;
        }

        let tangent = normalize(Coord {
            x: pts[i + 1].x - pts[i].x,
            y: pts[i + 1].y - pts[i].y,
        });

        result.push(tangent);
    }

    result
}

fn cumulative_lengths(pts: &[Coord]) -> Vec<f64> {
    let mut result = Vec::with_capacity(pts.len());
    let mut total = 0.0;
    result.push(0.0);
    for window in pts.windows(2) {
        total += Euclidean.distance(window[0], window[1]);
        result.push(total);
    }
    result
}

fn position_at(pts: &[Coord], cum: &[f64], dist: f64) -> Option<(Coord, Coord)> {
    if pts.len() < 2 {
        return None;
    }

    if dist <= 0.0 {
        let tangent = normalize(Coord {
            x: pts[1].x - pts[0].x,
            y: pts[1].y - pts[0].y,
        });
        return Some((pts[0], tangent));
    }

    if let Some(total) = cum.last() {
        if dist >= *total {
            let len = pts.len();
            let tangent = normalize(Coord {
                x: pts[len - 1].x - pts[len - 2].x,
                y: pts[len - 1].y - pts[len - 2].y,
            });
            return Some((pts[len - 1], tangent));
        }
    }

    let mut idx = 0;
    while idx + 1 < cum.len() && cum[idx + 1] < dist {
        idx += 1;
    }

    let seg_len = cum[idx + 1] - cum[idx];
    if seg_len == 0.0 {
        return None;
    }

    let t = (dist - cum[idx]) / seg_len;
    let p1 = pts[idx];
    let p2 = pts[idx + 1];
    let pos = Euclidean.point_at_ratio_between(p1.into(), p2.into(), t).0;
    let tangent = normalize(Coord {
        x: p2.x - p1.x,
        y: p2.y - p1.y,
    });

    Some((pos, tangent))
}

fn make_cluster_glyph_string(
    glyph_item: &GlyphItem,
    start_glyph: i32,
    end_glyph: i32,
) -> GlyphString {
    let src = glyph_item.glyph_string();
    let count = (end_glyph - start_glyph) as usize;
    let mut dst = GlyphString::new();
    dst.set_size(count as i32);

    for i in 0..count {
        let src_info = &src.glyph_info()[start_glyph as usize + i];
        let dst_info = &mut dst.glyph_info_mut()[i];

        dst_info.set_glyph(src_info.glyph());

        let src_geom = src_info.geometry();
        let dst_geom = dst_info.geometry_mut();

        dst_geom.set_width(src_geom.width());
        dst_geom.set_x_offset(src_geom.x_offset());
        dst_geom.set_y_offset(src_geom.y_offset());

        dst.log_clusters_mut()[i] = i as i32;
    }

    dst
}

fn collect_clusters(layout: &Layout) -> Vec<(f64, GlyphString, Font)> {
    let mut result = Vec::new();
    let ps = 1.0 / SCALE as f64;

    for line_idx in 0..layout.line_count() {
        let Some(line) = layout.line(line_idx) else {
            continue;
        };

        for run in line.runs() {
            let font = run.item().analysis().font();
            let glyphs = run.glyph_string();
            let infos = glyphs.glyph_info();
            let clusters = glyphs.log_clusters();

            if infos.is_empty() || clusters.is_empty() || infos.len() != clusters.len() {
                continue;
            }

            let mut start = 0usize;
            while start < infos.len() {
                let cluster_id = clusters[start];
                let mut end = start + 1;
                while end < infos.len() && clusters[end] == cluster_id {
                    end += 1;
                }

                let glyph_string = make_cluster_glyph_string(&run, start as i32, end as i32);
                let advance = glyph_string.glyph_info()[..]
                    .iter()
                    .map(|g| g.geometry().width() as f64 * ps)
                    .sum();

                result.push((advance, glyph_string, font.clone()));

                start = end;
            }
        }
    }

    result
}

fn draw_label(
    cr: &cairo::Context,
    glyphs: &[(GlyphString, Font, Coord, f64)],
    opts: &TextOnLineOptions,
) {
    if glyphs.is_empty() {
        return;
    }

    let ps = 1.0 / SCALE as f64;

    cr.save().unwrap();
    cr.push_group();

    for (glyph_string, font, pos, angle) in glyphs {
        // Rotate around the glyph's centroid (center of logical bbox).
        let mut gs = glyph_string.clone();
        let (_, logical) = gs.extents(font);
        let cx = (logical.x() as f64 + logical.width() as f64 / 2.0) * ps;
        let cy = (logical.y() as f64 + logical.height() as f64 / 2.0) * ps;

        cr.save().unwrap();
        cr.translate(pos.x, pos.y);
        cr.rotate(*angle);
        cr.translate(-cx, -cy);

        glyph_string_path(cr, font, &mut gs);

        cr.restore().unwrap();
    }

    cr.set_source_color_a(opts.halo_color, opts.halo_opacity);
    cr.set_dash(&[], 0.0);
    cr.set_line_width(opts.halo_width * 2.0);
    cr.stroke_preserve().unwrap();

    cr.set_source_color(opts.color);
    cr.fill().unwrap();

    cr.pop_group_to_source().unwrap();
    cr.paint_with_alpha(opts.alpha).unwrap();

    cr.restore().unwrap();
}

fn label_offsets(
    total_length: f64,
    total_advance: f64,
    spacing: f64,
    repeat_distance: Option<f64>,
    align: Align,
) -> Vec<f64> {
    if total_length < total_advance {
        return Vec::new();
    }

    if matches!(align, Align::Justify) {
        return vec![0.0];
    }

    // Step between label starts: either requested repeat distance or just "one label".
    let step = repeat_distance
        .map(|d| d.max(total_advance + spacing))
        .unwrap_or(total_length + spacing);

    // How many full labels can we fit.
    let count = if repeat_distance.is_some() {
        ((total_length - total_advance) / step).floor() as usize + 1
    } else {
        1
    };

    let total_span = if count > 0 {
        total_advance + step * (count.saturating_sub(1)) as f64
    } else {
        0.0
    };

    let start = match align {
        Align::Left => 0.0,
        Align::Center => ((total_length - total_span) / 2.0).max(0.0),
        Align::Right => (total_length - total_span).max(0.0),
        Align::Justify => 0.0,
    };

    (0..count).map(|i| start + i as f64 * step).collect()
}

pub fn text_on_line(
    context: &Context,
    line_string: &LineString,
    text: &str,
    mut collision: Option<&mut Collision<f64>>,
    options: &TextOnLineOptions,
) {
    let mut pts: Vec<Coord> = line_string.into_iter().copied().collect();

    pts.dedup_by(|a, b| a == b);

    if pts.len() < 2 {
        return;
    }

    let cum = cumulative_lengths(&pts);
    let total_length = *cum.last().unwrap_or(&0.0);

    if total_length == 0.0 {
        return;
    }

    let TextOnLineOptions {
        spacing,
        repeat_distance,
        align,
        upright,
        max_curvature_degrees,
        concave_spacing_factor,
        flo,
        ..
    } = options;

    let layout = create_pango_layout(context, text, flo);

    layout.set_width(-1); // no width constraint, so no wrapping happens at all

    let clusters = collect_clusters(&layout);
    if clusters.is_empty() {
        return;
    }

    let base_total_advance: f64 = clusters.iter().map(|c| c.0).sum();
    if base_total_advance == 0.0 {
        return;
    }

    let (advance_scale, extra_spacing_per_glyph) = match align {
        Align::Justify => ((total_length / base_total_advance).max(0.0), 0.0),
        _ => (1.0, 0.0),
    };

    let total_advance =
        base_total_advance * advance_scale + extra_spacing_per_glyph * clusters.len() as f64;

    let offsets = label_offsets(
        total_length,
        total_advance,
        *spacing,
        *repeat_distance,
        *align,
    );
    if offsets.is_empty() {
        return;
    }

    let mut placements: Vec<Vec<(GlyphString, Font, Coord, f64)>> = Vec::new();

    'outer: for start in offsets {
        // Decide per-repeat if we need to flip to stay upright.
        let overall_span_start = start;
        let overall_span_end = start + total_advance;
        let overall_tangent =
            weighted_tangent_for_span(&pts, &cum, overall_span_start, overall_span_end)
                .unwrap_or(Coord { x: 1.0, y: 0.0 });

        let base_angle = overall_tangent.y.atan2(overall_tangent.x);
        let adjusted_angle = adjust_upright_angle(base_angle, *upright);
        let flip_needed = (normalize_angle(adjusted_angle - base_angle)).abs() > PI / 2.0;
        let flip_offset = if flip_needed {
            0.0
        } else {
            normalize_angle(adjusted_angle - base_angle)
        };

        let mut pts_use = pts.clone();
        if flip_needed {
            pts_use.reverse();
        }
        let cum_use = cumulative_lengths(&pts_use);
        let start_use = if flip_needed {
            (total_length - total_advance - start).max(0.0)
        } else {
            start
        };

        let mut offset = start_use;
        let mut label_placements = Vec::new();
        let mut glyph_bboxes: Vec<Rect<f64>> = Vec::new();

        for (advance, glyph_string, font) in clusters.iter() {
            let eff_advance = *advance * advance_scale + extra_spacing_per_glyph;
            let span_start = offset;
            let span_end = offset + eff_advance;
            if span_end > total_length && !matches!(align, Align::Justify) {
                continue 'outer;
            }

            let (_, tangent) = match position_at(&pts_use, &cum_use, span_start + eff_advance / 2.0)
            {
                Some(v) => v,
                None => {
                    continue 'outer;
                }
            };

            let weighted_tangent =
                weighted_tangent_for_span(&pts_use, &cum_use, span_start, span_end)
                    .unwrap_or(tangent);

            let tangent_before = position_at(&pts_use, &cum_use, span_start.max(0.0))
                .map(|(_, t)| t)
                .unwrap_or(weighted_tangent);

            let tangent_after = position_at(&pts_use, &cum_use, span_end.min(total_length))
                .map(|(_, t)| t)
                .unwrap_or(weighted_tangent);

            let mut max_bend = angle_between(tangent_before, tangent_after);

            for pair in tangents_for_span(&pts_use, &cum_use, span_start, span_end).windows(2) {
                max_bend = max_bend.max(angle_between(pair[0], pair[1]));
            }

            if max_bend > *max_curvature_degrees {
                continue 'outer;
            }

            // Extra space proportional to curvature to avoid glyph tops touching on bends.
            let ratio = (max_bend / 180.0).clamp(0.0, 1.0);
            let concave_spacing = eff_advance * *concave_spacing_factor * ratio;

            let shifted_start = span_start + concave_spacing;
            let shifted_end = shifted_start + eff_advance;
            let shifted_center = shifted_start + eff_advance / 2.0;

            if shifted_end > total_length && !matches!(align, Align::Justify) {
                continue 'outer;
            }

            let (pos, _) = match position_at(&pts_use, &cum_use, shifted_center) {
                Some(v) => v,
                None => {
                    continue 'outer;
                }
            };

            let weighted_tangent =
                weighted_tangent_for_span(&pts_use, &cum_use, shifted_start, shifted_end)
                    .unwrap_or(weighted_tangent);

            let angle = normalize_angle(weighted_tangent.y.atan2(weighted_tangent.x) + flip_offset);

            // Track an axis-aligned bbox for the rotated glyph.
            let mut gs_bbox = glyph_string.clone();
            let (_, logical) = gs_bbox.extents(font);
            let w = logical.width() as f64 / SCALE as f64;
            let h = logical.height() as f64 / SCALE as f64;
            let hw = w / 2.0;
            let hh = h / 2.0;
            let cos = angle.cos().abs();
            let sin = angle.sin().abs();
            let rx = hw * cos + hh * sin;
            let ry = hw * sin + hh * cos;
            let glyph_bbox = Rect::new((pos.x - rx, pos.y - ry), (pos.x + rx, pos.y + ry));
            glyph_bboxes.push(glyph_bbox);

            label_placements.push((glyph_string.clone(), font.clone(), pos, angle));

            offset += eff_advance + concave_spacing;
        }

        if let Some(col) = collision.as_deref_mut() {
            if glyph_bboxes.iter().any(|bb| col.collides(bb)) {
                continue 'outer;
            }
            for bb in glyph_bboxes {
                let _ = col.add(bb);
            }
        }

        placements.push(label_placements);
    }

    for label in placements {
        draw_label(context, &label, options);
    }
}
