//! Shared explanation geometry for native terminal graphics and standalone SVG.
use crate::workflows::AppError;
use josh_core::molecular::Modality;
use josh_explain::{Archive, Attribution};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Table, Wrap,
        canvas::{Canvas, Line as CanvasLine, Points},
    },
};
use std::{collections::BTreeMap, f64::consts::TAU};

pub fn color(modalities: &[Modality]) -> Color {
    match modalities {
        [Modality::CopyNumber] => Color::Rgb(58, 184, 127),
        [Modality::Mutation] => Color::Rgb(240, 101, 103),
        [Modality::Signature] => Color::Rgb(71, 166, 225),
        [Modality::Demographic] => Color::Rgb(179, 185, 195),
        [Modality::Expression] => Color::Rgb(179, 133, 239),
        [Modality::Ihc] => Color::Rgb(238, 188, 91),
        [Modality::Histology] => Color::Rgb(79, 207, 202),
        _ => Color::Rgb(170, 170, 170),
    }
}
fn hex(modalities: &[Modality]) -> String {
    match color(modalities) {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => "#aaaaaa".into(),
    }
}
pub fn ranked(a: &Archive) -> Vec<&Attribution> {
    let mut rows: Vec<_> = a
        .result
        .as_ref()
        .map(|r| r.attributions.iter().collect())
        .unwrap_or_default();
    rows.sort_by(|a, b| {
        b.contribution
            .abs()
            .total_cmp(&a.contribution.abs())
            .then(a.group.cmp(&b.group))
    });
    rows
}
fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray))
}
fn safe(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}
fn xml(text: &str) -> String {
    safe(text)
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn label(modalities: &[Modality]) -> String {
    modalities
        .iter()
        .map(|m| m.label())
        .collect::<Vec<_>>()
        .join(" + ")
}
fn ring_rows(a: &Archive) -> Vec<&Attribution> {
    let mut rows = ranked(a);
    rows.sort_by(|a, b| a.modalities.cmp(&b.modalities).then(a.group.cmp(&b.group)));
    rows
}
fn ring_points(start: f64, end: f64, inner: f64, outer: f64) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    // Sample only this annular sector; total work follows circle area instead
    // of scanning a complete image once per feature.
    let angles = ((end - start) * outer * 200.0).ceil() as usize;
    let radii = ((outer - inner) * 200.0).ceil() as usize;
    for a in 0..angles {
        let angle = start + (end - start) * a as f64 / angles as f64;
        let (sin, cos) = angle.sin_cos();
        for r in 0..=radii {
            let radius = inner + (outer - inner) * r as f64 / radii.max(1) as f64;
            points.push((radius * cos, radius * sin));
        }
    }
    points
}
pub fn draw_ring(frame: &mut Frame, area: Rect, a: &Archive, selected: usize) {
    let rows = ring_rows(a);
    let total: f64 = rows.iter().map(|r| r.contribution.abs()).sum();
    if total <= f64::EPSILON {
        frame.render_widget(
            Paragraph::new("No nonzero attribution magnitude").block(panel(" Attribution ring ")),
            area,
        );
        return;
    }
    let focused = ranked(a).get(selected).map(|r| r.group.as_str());
    let mut start = 0.0;
    let sectors: Vec<_> = rows
        .iter()
        .map(|r| {
            let end = start + r.contribution.abs() / total * TAU;
            let points = ring_points(start, end, 0.52, 0.82);
            start = end;
            (r, points)
        })
        .collect();
    let mut categories: BTreeMap<Vec<Modality>, f64> = BTreeMap::new();
    for r in &rows {
        *categories.entry(r.modalities.clone()).or_default() += r.contribution.abs();
    }
    let mut start = 0.0;
    let outer: Vec<_> = categories
        .iter()
        .map(|(category, value)| {
            let end = start + value / total * TAU;
            let points = ring_points(start, end, 0.86, 0.98);
            start = end;
            (category, points)
        })
        .collect();
    let inner = panel("").inner(area);
    let aspect = (f64::from(inner.width) / f64::from(inner.height.max(1) * 2)).max(0.2);
    let bound_y = 1.15_f64.max(1.15 / aspect);
    let bound_x = bound_y * aspect;
    let canvas = Canvas::default()
        .block(panel(" Attribution magnitude · category / feature "))
        .marker(Marker::HalfBlock)
        .x_bounds([-bound_x, bound_x])
        .y_bounds([-bound_y, bound_y])
        .paint(|ctx| {
            for (r, points) in &sectors {
                ctx.draw(&Points {
                    coords: points,
                    color: if focused == Some(r.group.as_str()) {
                        Color::White
                    } else {
                        color(&r.modalities)
                    },
                });
            }
            for (category, points) in &outer {
                ctx.draw(&Points {
                    coords: points,
                    color: color(category),
                });
            }
            ctx.print(
                -0.30,
                0.10,
                Line::styled("|Shapley|", Style::default().fg(Color::White)),
            );
            ctx.print(
                -0.31,
                -0.16,
                Line::styled("magnitude", Style::default().fg(Color::Gray)),
            );
        });
    frame.render_widget(canvas, area);
}
/// A percentile axis is only used when every plotted group is a single numerical feature
/// and a compatible reference background exists. Otherwise feature rank is explicit.
fn scatter_x(a: &Archive, rows: &[&Attribution]) -> (Vec<f64>, String, (f64, f64)) {
    if let Some(background) = &a.background {
        let values: Option<Vec<f64>> = rows
            .iter()
            .map(|r| {
                if r.feature_ids.len() != 1 {
                    return None;
                }
                let id = &r.feature_ids[0];
                let f = a.features.features.iter().find(|f| &f.id == id)?;
                let x = f.value.as_ref()?.numeric()?;
                let vals: Option<Vec<f64>> = background
                    .samples
                    .iter()
                    .map(|s| {
                        s.features
                            .iter()
                            .find(|f| &f.id == id)?
                            .value
                            .as_ref()?
                            .numeric()
                    })
                    .collect();
                let vals = vals?;
                Some(
                    vals.iter()
                        .map(|v| {
                            if *v < x {
                                1.0
                            } else if *v == x {
                                0.5
                            } else {
                                0.0
                            }
                        })
                        .sum::<f64>()
                        / vals.len() as f64
                        * 100.0,
                )
            })
            .collect();
        if let Some(values) = values {
            return (
                values,
                "Feature percentile in development background".into(),
                (-5.0, 105.0),
            );
        }
    }
    (
        (1..=rows.len()).map(|i| i as f64).collect(),
        "Feature rank · raw values in inspector".into(),
        (0.0, rows.len() as f64 + 1.0),
    )
}
pub fn draw_scatter(frame: &mut Frame, area: Rect, a: &Archive, selected: usize) {
    let rows = ranked(a);
    let rows: Vec<_> = rows.into_iter().take(10).collect();
    let (xs, title, bounds) = scatter_x(a, &rows);
    let max = rows
        .iter()
        .map(|r| r.contribution.abs() * 100.0)
        .fold(1.0_f64, f64::max)
        * 1.4;
    let canvas = Canvas::default()
        .block(panel(" Signed contribution (percentage points) "))
        .marker(Marker::Braille)
        .x_bounds([bounds.0, bounds.1])
        .y_bounds([-max, max])
        .paint(|ctx| {
            ctx.draw(&CanvasLine {
                x1: bounds.0,
                y1: 0.0,
                x2: bounds.1,
                y2: 0.0,
                color: Color::DarkGray,
            });
            let xrange = bounds.1 - bounds.0;
            for (i, r) in rows.iter().enumerate() {
                let magnitude = (r.contribution.abs() * 100.0 / max).sqrt();
                // Area follows absolute contribution, with a small visibility floor.
                let radius = (0.04 * magnitude).max(0.003) * xrange;
                let y_radius = radius * 2.0 * max / xrange
                    * (f64::from(area.width.max(1)) / f64::from(area.height.max(1) * 2));
                let mut coords = Vec::new();
                for dy in -6..=6 {
                    for dx in -6..=6 {
                        if dx * dx + dy * dy <= 36 {
                            coords.push((
                                xs[i] + f64::from(dx) / 6.0 * radius,
                                r.contribution * 100.0 + f64::from(dy) / 6.0 * y_radius,
                            ));
                        }
                    }
                }
                ctx.draw(&Points {
                    coords: &coords,
                    color: if i == selected {
                        Color::White
                    } else {
                        color(&r.modalities)
                    },
                });
                ctx.print(
                    xs[i],
                    (r.contribution * 100.0 + max * 0.13).min(max * 0.87),
                    Line::styled(
                        format!("{} {}", i + 1, safe(&r.label)),
                        Style::default().fg(color(&r.modalities)),
                    ),
                );
            }
            ctx.print(bounds.0 + 0.1, -max * 0.90, Line::from(title.clone()));
            ctx.print(
                bounds.0 + 0.1,
                max * 0.93,
                Line::from(format!("Y range −{max:.1} to +{max:.1} pp")),
            );
        });
    frame.render_widget(canvas, area);
}
pub fn draw_table(frame: &mut Frame, area: Rect, a: &Archive, selected: usize) {
    let rows = ranked(a);
    let height = area.height.saturating_sub(4) as usize;
    let offset = selected.saturating_sub(height.saturating_sub(1));
    let max = rows
        .iter()
        .map(|r| r.contribution.abs())
        .fold(1e-12_f64, f64::max);
    let table = Table::new(
        rows.iter()
            .enumerate()
            .skip(offset)
            .take(height)
            .map(|(i, r)| {
                let bars = "▰".repeat((r.contribution.abs() / max * 12.0).round() as usize);
                Row::new(vec![
                    Cell::from(format!(
                        "{}{}",
                        if i == selected { "› " } else { "  " },
                        safe(&r.label)
                    )),
                    Cell::from(format!("{:+.3}", r.contribution * 100.0)),
                    Cell::from(format!(
                        "{}{}",
                        if r.contribution < 0.0 { "−" } else { "+" },
                        bars
                    )),
                    Cell::from(label(&r.modalities)),
                ])
                .style(Style::default().fg(if i == selected {
                    Color::White
                } else {
                    color(&r.modalities)
                }))
            }),
        [
            Constraint::Percentage(36),
            Constraint::Length(11),
            Constraint::Length(15),
            Constraint::Min(8),
        ],
    )
    .header(
        Row::new(["Feature / group", "Δ score pp", "Direction", "Category"])
            .style(Style::default().fg(Color::White).bold()),
    )
    .block(panel(" Contributions · arrows select "));
    frame.render_widget(table, area);
}

/// A dedicated paired figure, independent of the older chart/table layout.
/// Geometry always represents this archive's probability-space Shapley game.
pub fn draw_onconpc(frame: &mut Frame, area: Rect, a: &Archive, selected: usize) {
    let Some(result) = &a.result else { return };
    let rows = ranked(a);
    let selected = selected.min(rows.len().saturating_sub(1));
    let compact = area.height < 22;
    let [heading, plots, detail] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(if compact { 2 } else { 6 }),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                format!(
                    "Explain: {} ({}) · raw score {:.3}",
                    safe(crate::report::class_label(
                        &a.inference,
                        &result.target_class
                    )),
                    safe(&result.target_class),
                    result.full_probability
                ),
                Style::default().fg(crate::ui::TEXT).bold(),
            ),
            Line::styled(
                format!(
                    "{} · {}",
                    crate::report::source_label(a.inference.source),
                    if a.inference.status == "abstained" {
                        "ABSTAINED"
                    } else {
                        "REVIEW REQUIRED"
                    }
                ),
                Style::default().fg(crate::ui::GOLD),
            ),
            Line::styled(
                "OncoNPC-inspired · Shapley Δ raw score (pp) · uncalibrated",
                Style::default().fg(crate::ui::MUTED),
            ),
        ]),
        heading,
    );
    if plots.width >= 72 {
        let [ring, scatter] =
            Layout::horizontal([Constraint::Percentage(43), Constraint::Percentage(57)])
                .areas(plots);
        draw_figure_ring(frame, ring, a, selected);
        draw_figure_scatter(frame, scatter, a, selected);
    } else if plots.height >= 18 {
        let [ring, scatter] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(plots);
        draw_figure_ring(frame, ring, a, selected);
        draw_figure_scatter(frame, scatter, a, selected);
    } else {
        draw_table(frame, plots, a, selected);
    }
    let sum: f64 = rows.iter().map(|r| r.contribution).sum();
    let total: f64 = rows.iter().map(|r| r.contribution.abs()).sum();
    let mut lines = Vec::new();
    if let Some(r) = rows.get(selected) {
        let values = r
            .feature_ids
            .iter()
            .filter_map(|id| a.features.features.iter().find(|f| &f.id == id))
            .map(|f| {
                format!(
                    "{} = {}",
                    safe(&f.name),
                    safe(
                        &f.value
                            .as_ref()
                            .map(|v| v.display())
                            .unwrap_or_else(|| "unavailable".into())
                    )
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        lines.push(Line::styled(
            format!(
                "#{} {}  {:+.3} pp · {}",
                selected + 1,
                safe(&r.label),
                r.contribution * 100.0,
                values
            ),
            Style::default().fg(color(&r.modalities)),
        ));
        if !compact {
            lines.push(Line::from(format!(
                "{} · sampling SE {} pp · {}",
                label(&r.modalities),
                r.sampling_standard_error
                    .map(|v| format!("{:.4}", v * 100.0))
                    .unwrap_or_else(|| "unavailable".into()),
                safe(&result.method)
            )));
            for f in r
                .feature_ids
                .iter()
                .filter_map(|id| a.features.features.iter().find(|f| &f.id == id))
                .take(2)
            {
                lines.push(Line::styled(
                    format!(
                        "{} · {} · source {} / record {}",
                        safe(&f.assay),
                        safe(&f.coverage),
                        safe(&f.source.source_id),
                        f.source.record
                    ),
                    Style::default().fg(crate::ui::MUTED),
                ));
            }
        }
    }
    if !compact {
        let shown = figure_rows(a, selected);
        let remaining: Vec<_> = rows
            .iter()
            .filter(|r| !shown.iter().any(|(_, s)| s.group == r.group))
            .collect();
        lines.push(Line::from(format!("Ring: all {} groups, |Δ| {:.2} pp · scatter: {} · omitted: {} (net {:+.2}, |Δ| {:.2} pp)",
            rows.len(), total * 100.0, shown.len(), remaining.len(),
            remaining.iter().map(|r| r.contribution).sum::<f64>() * 100.0,
            remaining.iter().map(|r| r.contribution.abs()).sum::<f64>() * 100.0)));
    }
    lines.push(Line::styled(
        format!(
            "Base {:.3} + Δ {:+.3} = {:.3} · residual {:.1e}",
            result.baseline_probability, sum, result.full_probability, result.additivity_residual
        ),
        Style::default().fg(crate::ui::MUTED),
    ));
    frame.render_widget(Paragraph::new(lines), detail);
}

fn figure_rows(a: &Archive, selected: usize) -> Vec<(usize, &Attribution)> {
    let rows = ranked(a);
    let mut visible: Vec<_> = rows.iter().copied().enumerate().take(10).collect();
    // Keep a selected low-ranked feature visible without misrepresenting its rank.
    if selected >= 10
        && let Some(r) = rows.get(selected)
    {
        visible.pop();
        visible.push((selected, *r));
    }
    visible
}

fn draw_figure_ring(frame: &mut Frame, area: Rect, a: &Archive, selected: usize) {
    let block = crate::ui::panel(" Category / feature ring ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = ring_rows(a);
    let ranked = ranked(a);
    let total: f64 = rows.iter().map(|r| r.contribution.abs()).sum();
    if total <= f64::EPSILON {
        frame.render_widget(Paragraph::new("No nonzero attribution magnitude"), inner);
        return;
    }
    let mut categories: BTreeMap<Vec<Modality>, f64> = BTreeMap::new();
    for r in &rows {
        *categories.entry(r.modalities.clone()).or_default() += r.contribution.abs();
    }
    let legend_height = if inner.height >= 12 {
        categories.len().min(4) as u16
    } else {
        0
    };
    let [circle, legend] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(legend_height)]).areas(inner);
    let aspect = (f64::from(circle.width) / f64::from(circle.height.max(1) * 2)).max(0.1);
    let by = 1.06_f64.max(1.06 / aspect);
    let bx = by * aspect;
    let focused = ranked.get(selected).map(|r| r.group.as_str());
    let mut start = std::f64::consts::FRAC_PI_2;
    let sectors: Vec<_> = rows
        .iter()
        .map(|r| {
            let end = start + r.contribution.abs() / total * TAU;
            let gap = ((end - start) * 0.06).min(0.025);
            let points = ring_points(start + gap, end - gap, 0.52, 0.82);
            let middle = (start + end) / 2.0;
            let highlight = if focused == Some(r.group.as_str()) {
                ring_points(start + gap, end - gap, 0.82, 0.85)
            } else {
                Vec::new()
            };
            start = end;
            (r, points, middle, highlight)
        })
        .collect();
    let mut start = std::f64::consts::FRAC_PI_2;
    let outer: Vec<_> = categories
        .iter()
        .map(|(m, value)| {
            let end = start + value / total * TAU;
            let points = ring_points(start, end, 0.87, 1.0);
            start = end;
            (m, points)
        })
        .collect();
    frame.render_widget(
        Canvas::default()
            .marker(Marker::HalfBlock)
            .background_color(crate::ui::PANEL)
            .x_bounds([-bx, bx])
            .y_bounds([-by, by])
            .paint(|ctx| {
                for (r, points, _, highlight) in &sectors {
                    let shade = if focused == Some(r.group.as_str()) {
                        color(&r.modalities)
                    } else {
                        match color(&r.modalities) {
                            Color::Rgb(r, g, b) => Color::Rgb(r / 2, g / 2, b / 2),
                            c => c,
                        }
                    };
                    ctx.draw(&Points {
                        coords: points,
                        color: shade,
                    });
                    ctx.draw(&Points {
                        coords: highlight,
                        color: Color::White,
                    });
                }
                for (m, points) in &outer {
                    ctx.draw(&Points {
                        coords: points,
                        color: color(m),
                    });
                }
                // Numeric labels link sectors to the scatter and inspector; tiny sectors stay unlabeled.
                if circle.height >= 9 {
                    for (r, _, angle, _) in &sectors {
                        if r.contribution.abs() / total >= 0.045 {
                            let rank =
                                ranked.iter().position(|s| s.group == r.group).unwrap_or(0) + 1;
                            ctx.print(
                                0.69 * angle.cos() - 0.03,
                                0.69 * angle.sin(),
                                Line::styled(
                                    rank.to_string(),
                                    Style::default().fg(crate::ui::TEXT).bold(),
                                ),
                            );
                        }
                    }
                }
                ctx.print(
                    -0.28,
                    0.08,
                    Line::styled("|Shapley|", Style::default().fg(crate::ui::TEXT)),
                );
                ctx.print(
                    -0.22,
                    -0.16,
                    Line::styled(
                        format!("#{:02}", selected + 1),
                        Style::default().fg(crate::ui::ACCENT).bold(),
                    ),
                );
            }),
        circle,
    );
    let lines = categories
        .iter()
        .map(|(m, value)| {
            Line::from(vec![
                Span::styled("● ", Style::default().fg(color(m))),
                Span::raw(format!("{} {:>5.1}%", label(m), value / total * 100.0)),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), legend);
}

fn draw_figure_scatter(frame: &mut Frame, area: Rect, a: &Archive, selected: usize) {
    let block = crate::ui::panel(" Signed contribution · pp ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width < 12 || inner.height < 3 {
        return;
    }
    let visible = figure_rows(a, selected);
    let rows: Vec<_> = visible.iter().map(|(_, r)| *r).collect();
    let (mut xs, _, mut bounds) = scatter_x(a, &rows);
    let percentile = bounds.1 == 105.0;
    if !percentile {
        xs = visible.iter().map(|(i, _)| (*i + 1) as f64).collect();
        bounds = (0.0, xs.iter().copied().fold(1.0_f64, f64::max) + 1.0);
    }
    let plot = Rect::new(inner.x + 6, inner.y, inner.width - 7, inner.height - 2);
    let ymax = rows
        .iter()
        .map(|r| r.contribution.abs() * 100.0)
        .fold(1.0_f64, f64::max)
        * 1.35;
    let sx = (bounds.1 - bounds.0) / f64::from(plot.width.saturating_sub(1).max(1));
    let sy = 2.0 * ymax / f64::from(plot.height.saturating_sub(1).max(1));
    // Place labels in terminal-cell coordinates to keep them inside the plot and apart.
    let mut occupied: Vec<Rect> = Vec::new();
    let mut labels = Vec::new();
    for ((rank, r), x) in visible.iter().zip(&xs) {
        let limit = usize::from((plot.width / 2).max(5));
        let name = format!("{} {}", rank + 1, safe(&r.label));
        let name = truncate_label(&name, limit);
        let width = Line::from(name.clone())
            .width()
            .min(usize::from(plot.width)) as u16;
        let col = (((x - bounds.0) / sx).round() as u16).min(plot.width.saturating_sub(width));
        let row = (((ymax - r.contribution * 100.0) / sy).round() as u16)
            .min(plot.height.saturating_sub(1));
        let slot = (0..plot.height).min_by_key(|y| {
            let rect = Rect::new(col.saturating_sub(1), *y, width + 2, 1);
            let overlaps = occupied.iter().any(|r| r.intersects(rect));
            (overlaps, y.abs_diff(row.saturating_sub(1)))
        });
        if let Some(y) = slot {
            let rect = Rect::new(col.saturating_sub(1), y, width + 2, 1);
            if !occupied.iter().any(|r| r.intersects(rect)) {
                occupied.push(rect);
                labels.push((Rect::new(col, y, width, 1), name, r));
            }
        }
    }
    frame.render_widget(
        Canvas::default()
            .marker(Marker::Braille)
            .background_color(crate::ui::PANEL)
            .x_bounds([bounds.0, bounds.1])
            .y_bounds([-ymax, ymax])
            .paint(|ctx| {
                for y in [-ymax, 0.0, ymax] {
                    ctx.draw(&CanvasLine {
                        x1: bounds.0,
                        y1: y,
                        x2: bounds.1,
                        y2: y,
                        color: if y == 0.0 {
                            crate::ui::MUTED
                        } else {
                            crate::ui::RAISED
                        },
                    });
                }
                let max = rows
                    .iter()
                    .map(|r| r.contribution.abs())
                    .fold(1e-12_f64, f64::max);
                for ((rank, r), x) in visible.iter().zip(&xs) {
                    let radius = (r.contribution.abs() / max).sqrt() * 1.3 + 0.35;
                    let mut dots = Vec::new();
                    for dy in -8..=8 {
                        for dx in -8..=8 {
                            if dx * dx + dy * dy <= 64 {
                                dots.push((
                                    *x + f64::from(dx) / 8.0 * radius * sx,
                                    r.contribution * 100.0
                                        + f64::from(dy) / 8.0 * radius * sy / 2.0,
                                ));
                            }
                        }
                    }
                    ctx.draw(&Points {
                        coords: &dots,
                        color: if *rank == selected {
                            Color::White
                        } else {
                            color(&r.modalities)
                        },
                    });
                    if let Some((rect, _, _)) = labels.iter().find(|(_, _, s)| s.group == r.group) {
                        ctx.draw(&CanvasLine {
                            x1: *x,
                            y1: r.contribution * 100.0,
                            x2: bounds.0 + f64::from(rect.x) * sx,
                            y2: ymax - f64::from(rect.y) * sy,
                            color: crate::ui::MUTED,
                        });
                    }
                }
            }),
        plot,
    );
    // Render labels at exact cells: converting coordinates through Canvas::print
    // can round adjacent rows into the same row and defeat collision avoidance.
    for (rect, text, r) in labels {
        frame.render_widget(
            Paragraph::new(text).style(
                Style::default()
                    .fg(color(&r.modalities))
                    .bg(crate::ui::PANEL),
            ),
            Rect::new(plot.x + rect.x, plot.y + rect.y, rect.width, 1),
        );
    }
    for (row, value) in [
        (0, ymax),
        (plot.height.saturating_sub(1) / 2, 0.0),
        (plot.height.saturating_sub(1), -ymax),
    ] {
        frame.render_widget(
            Paragraph::new(format!("{value:>+5.1}")).style(Style::default().fg(crate::ui::MUTED)),
            Rect::new(inner.x, plot.y + row, 6, 1),
        );
    }
    let ticks: Vec<f64> = if percentile {
        vec![0.0, 50.0, 100.0]
    } else {
        xs.clone()
    };
    let mut last_end = 0;
    for value in ticks {
        let col = (((value - bounds.0) / sx).round() as u16).min(plot.width.saturating_sub(3));
        if col < last_end {
            continue;
        }
        frame.render_widget(
            Paragraph::new(format!("{value:.0}")).style(Style::default().fg(crate::ui::MUTED)),
            Rect::new(plot.x + col, plot.bottom(), 3, 1),
        );
        last_end = col + 3;
    }
    frame.render_widget(
        Paragraph::new(if percentile {
            "X: background percentile"
        } else {
            "X: feature rank (raw values below)"
        })
        .style(Style::default().fg(crate::ui::MUTED)),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

fn truncate_label(text: &str, max_width: usize) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let mut next = out.clone();
        next.push(c);
        if Line::from(next.clone()).width() >= max_width {
            out.push('…');
            return out;
        }
        out = next;
    }
    out
}

pub fn draw(frame: &mut Frame, area: Rect, a: &Archive, selected: usize, page: usize) {
    let Some(result) = &a.result else {
        frame.render_widget(
            Paragraph::new("Explanation incomplete. Successful evaluations are saved for resume.")
                .block(panel(" Explanation ")),
            area,
        );
        return;
    };
    if area.width < 90 || area.height < 23 {
        draw_table(frame, area, a, selected);
        return;
    }
    let [charts, table, detail] = Layout::vertical([
        Constraint::Min(12),
        Constraint::Length(9),
        Constraint::Length(5),
    ])
    .areas(area);
    if page == 2 {
        draw_waterfall(frame, charts, a);
    } else if charts.width >= 115 {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)])
                .areas(charts);
        draw_ring(frame, left, a, selected);
        draw_scatter(frame, right, a, selected);
    } else if page.is_multiple_of(2) {
        draw_ring(frame, charts, a, selected);
    } else {
        draw_scatter(frame, charts, a, selected);
    }
    draw_table(frame, table, a, selected);
    let rows = ranked(a);
    let selected = rows.get(selected).or_else(|| rows.first());
    let description=selected.map(|r|{
        let values=r.feature_ids.iter().filter_map(|id|a.features.features.iter().find(|f|&f.id==id)).map(|f|format!("{}: {} · {} · {} · source {} record {}",safe(&f.name),safe(&f.value.as_ref().map(|v|v.display()).unwrap_or_default()),safe(&f.assay),safe(&f.coverage),safe(&f.source.source_id),f.source.record)).collect::<Vec<_>>().join("; ");
        format!("{}\n{} | Δ {:+.3} pp | sampling SE {} pp\nBase {:.3} + contributions {:+.3} = {:.3}; residual {:.1e}",values,result.method,r.contribution*100.0,r.sampling_standard_error.map(|e|format!("{:.4}",e*100.0)).unwrap_or_else(||"n/a (exact or one pair)".into()),result.baseline_probability,result.attributions.iter().map(|x|x.contribution).sum::<f64>(),result.full_probability,result.additivity_residual)
    }).unwrap_or_default();
    frame.render_widget(
        Paragraph::new(description)
            .wrap(Wrap { trim: false })
            .block(panel(" Selected evidence and numerical reconciliation ")),
        detail,
    );
}
pub fn draw_waterfall(frame: &mut Frame, area: Rect, a: &Archive) {
    let Some(result) = &a.result else {
        return;
    };
    let rows = ranked(a);
    let mut steps: Vec<_> = rows
        .iter()
        .take(10)
        .map(|r| (r.label.clone(), r.contribution, color(&r.modalities)))
        .collect();
    if rows.len() > 10 {
        steps.push((
            "Remainder".into(),
            rows.iter().skip(10).map(|r| r.contribution).sum(),
            Color::Gray,
        ));
    }
    let mut cumulative = result.baseline_probability * 100.0;
    let mut bars = vec![("Base".to_owned(), 0.0, cumulative, Color::Gray)];
    for (name, value, color) in steps {
        let next = cumulative + value * 100.0;
        bars.push((name, cumulative, next, color));
        cumulative = next;
    }
    bars.push((
        "Full".into(),
        0.0,
        result.full_probability * 100.0,
        Color::White,
    ));
    let min = bars
        .iter()
        .map(|(_, a, b, _)| a.min(*b))
        .fold(0.0_f64, f64::min);
    let max = bars
        .iter()
        .map(|(_, a, b, _)| a.max(*b))
        .fold(1.0_f64, f64::max);
    let span = (max - min).max(1.0);
    let label_chars = (area.width.saturating_sub(2) as usize / (bars.len() + 1))
        .saturating_sub(2)
        .max(3);
    frame.render_widget(
        Canvas::default()
            .block(panel(
                " Waterfall · baseline + signed contributions = raw score ",
            ))
            .marker(Marker::HalfBlock)
            .x_bounds([-0.6, bars.len() as f64 - 0.3])
            .y_bounds([min - span * 0.15, max + span * 0.2])
            .paint(|ctx| {
                for (i, (name, start, end, color)) in bars.iter().enumerate() {
                    let mut coords = Vec::new();
                    for x in -12..=12 {
                        for y in 0..=80 {
                            coords.push((
                                i as f64 + f64::from(x) / 40.0,
                                start + (end - start) * f64::from(y) / 80.0,
                            ));
                        }
                    }
                    ctx.draw(&Points {
                        coords: &coords,
                        color: *color,
                    });
                    ctx.print(
                        i as f64 - 0.3,
                        min - span * 0.1,
                        ratatui::text::Line::from(if name.chars().count() > label_chars {
                            format!(
                                "{}…",
                                safe(name).chars().take(label_chars - 1).collect::<String>()
                            )
                        } else {
                            safe(name)
                        }),
                    );
                    ctx.print(
                        i as f64 - 0.3,
                        start.max(*end) + span * 0.08,
                        ratatui::text::Line::from(format!("{end:.1}")),
                    );
                }
            }),
        area,
    );
}
pub fn csv(a: &Archive) -> Result<String, AppError> {
    let result = a.result.as_ref().ok_or("incomplete explanation")?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "target_class",
        "group",
        "feature_ids",
        "label",
        "category",
        "contribution_probability",
        "contribution_pp",
        "sampling_se_probability",
        "baseline_probability",
        "full_probability",
        "method",
    ])?;
    for r in &result.attributions {
        writer.write_record([
            result.target_class.clone(),
            r.group.clone(),
            r.feature_ids.join(";"),
            r.label.clone(),
            label(&r.modalities),
            r.contribution.to_string(),
            (r.contribution * 100.0).to_string(),
            r.sampling_standard_error
                .map(|x| x.to_string())
                .unwrap_or_default(),
            result.baseline_probability.to_string(),
            result.full_probability.to_string(),
            result.method.clone(),
        ])?;
    }
    Ok(String::from_utf8(writer.into_inner()?)?)
}
fn arc(cx: f64, cy: f64, inner: f64, outer: f64, start: f64, end: f64) -> String {
    // Split nearly full circles so SVG arc endpoints never coincide.
    let end = end.min(start + TAU - 1e-6);
    let large = usize::from(end - start > std::f64::consts::PI);
    let (x1, y1) = (cx + outer * start.cos(), cy - outer * start.sin());
    let (x2, y2) = (cx + outer * end.cos(), cy - outer * end.sin());
    let (x3, y3) = (cx + inner * end.cos(), cy - inner * end.sin());
    let (x4, y4) = (cx + inner * start.cos(), cy - inner * start.sin());
    format!(
        "M{x1:.3},{y1:.3} A{outer},{outer} 0 {large} 0 {x2:.3},{y2:.3} L{x3:.3},{y3:.3} A{inner},{inner} 0 {large} 1 {x4:.3},{y4:.3} Z"
    )
}
pub fn svg(a: &Archive) -> Result<String, AppError> {
    use std::fmt::Write as _;
    let r = a.result.as_ref().ok_or("incomplete explanation")?;
    let rows = ranked(a);
    let top: Vec<_> = rows.iter().copied().take(10).collect();
    let total: f64 = rows.iter().map(|x| x.contribution.abs()).sum();
    let mut out = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1200\" height=\"900\" viewBox=\"0 0 1200 900\"><rect width=\"1200\" height=\"900\" fill=\"#101824\"/><g font-family=\"sans-serif\" fill=\"#e5edf5\">",
    );
    let badge = match a.inference.source {
        josh_core::Source::Mock => "ANALYTICAL DEMO — NO CANCER PREDICTION",
        josh_core::Source::Replay => "UNVERIFIED REPLAY — RAW UNCALIBRATED SCORE",
        josh_core::Source::Jev => "JEV RESEARCH — RAW UNCALIBRATED SCORE",
    };
    writeln!(
        out,
        "<text x=\"30\" y=\"35\" font-size=\"18\">{badge}</text><text x=\"30\" y=\"65\" font-size=\"22\">{} · raw score {:.4}</text><text x=\"30\" y=\"90\" font-size=\"12\">{}</text>",
        xml(&r.target_class),
        r.full_probability,
        xml(&r.method)
    )?;
    writeln!(
        out,
        "<text x=\"650\" y=\"35\" font-size=\"12\">{} · {} · run {}</text>",
        xml(&a.features.sample_id),
        xml(&a.inference.status),
        xml(&a.inference.request_sha256[..12])
    )?;
    let mut start = 0.0;
    let mut categories: BTreeMap<Vec<Modality>, f64> = BTreeMap::new();
    if total > f64::EPSILON {
        for row in ring_rows(a) {
            let end = start + row.contribution.abs() / total * TAU;
            writeln!(
                out,
                "<path d=\"{}\" fill=\"{}\" stroke=\"#101824\"><title>{}: {:+.4} pp</title></path>",
                arc(270.0, 310.0, 90.0, 163.0, start, end),
                hex(&row.modalities),
                xml(&row.label),
                row.contribution * 100.0
            )?;
            start = end;
            *categories.entry(row.modalities.clone()).or_default() += row.contribution.abs();
        }
        start = 0.0;
        for (category, value) in &categories {
            let end = start + value / total * TAU;
            writeln!(
                out,
                "<path d=\"{}\" fill=\"{}\"><title>{}</title></path>",
                arc(270.0, 310.0, 172.0, 188.0, start, end),
                hex(category),
                xml(&label(category))
            )?;
            start = end;
        }
        for (i, category) in categories.keys().enumerate() {
            writeln!(
                out,
                "<text x=\"465\" y=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
                205 + i * 20,
                hex(category),
                xml(&label(category))
            )?;
        }
    } else {
        writeln!(
            out,
            "<text x=\"100\" y=\"390\" font-size=\"13\">No nonzero attribution magnitude</text>"
        )?;
    }
    writeln!(
        out,
        "<text x=\"215\" y=\"310\" font-size=\"19\">|Shapley|</text><text x=\"205\" y=\"334\" font-size=\"13\">attribution magnitude</text>"
    )?;
    let (xs, xlabel, bounds) = scatter_x(a, &top);
    let ymax = top
        .iter()
        .map(|v| v.contribution.abs() * 100.0)
        .fold(1.0_f64, f64::max)
        * 1.3;
    writeln!(
        out,
        "<path d=\"M640 140 V490 H1160 M640 315 H1160\" fill=\"none\" stroke=\"#627285\"/><text x=\"640\" y=\"125\" font-size=\"14\">Signed contribution (percentage points)</text><text x=\"660\" y=\"525\" font-size=\"13\">{}</text>",
        xml(&xlabel)
    )?;
    for y in [-ymax, 0.0, ymax] {
        writeln!(
            out,
            "<text x=\"596\" y=\"{}\" font-size=\"12\">{y:+.1}</text>",
            315.0 - y / ymax * 175.0
        )?;
    }
    for (i, row) in top.iter().enumerate() {
        let x = 650.0 + (xs[i] - bounds.0) / (bounds.1 - bounds.0) * 480.0;
        let y = 315.0 - row.contribution * 100.0 / ymax * 175.0;
        let radius = (15.0 * (row.contribution.abs() * 100.0 / ymax).sqrt()).max(1.0);
        writeln!(
            out,
            "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"{radius:.2}\" fill=\"{}\"/><text x=\"{:.2}\" y=\"{:.2}\" font-size=\"12\">{} {}</text><text x=\"{x:.2}\" y=\"510\" font-size=\"11\">{:.0}</text>",
            hex(&row.modalities),
            x - 20.0,
            y - 17.0,
            i + 1,
            xml(&row.label),
            xs[i]
        )?;
    }
    writeln!(
        out,
        "<text x=\"30\" y=\"565\" font-size=\"16\">Top contributions · positive supports / negative opposes the selected class</text>"
    )?;
    for (i, row) in top.iter().enumerate() {
        let y = 590 + i * 21;
        let values = row
            .feature_ids
            .iter()
            .filter_map(|id| a.features.features.iter().find(|f| &f.id == id))
            .map(|f| f.value.as_ref().map(|v| v.display()).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("; ");
        writeln!(
            out,
            "<text x=\"35\" y=\"{y}\" font-size=\"13\" fill=\"{}\">{}. {}</text><text x=\"335\" y=\"{y}\" font-size=\"13\">{:+.4} pp</text><text x=\"470\" y=\"{y}\" font-size=\"12\">{}</text>",
            hex(&row.modalities),
            i + 1,
            xml(&row.label),
            row.contribution * 100.0,
            xml(&values)
        )?;
    }
    let remainder: f64 = rows.iter().skip(10).map(|r| r.contribution).sum();
    let abs_remainder: f64 = rows.iter().skip(10).map(|r| r.contribution.abs()).sum();
    writeln!(
        out,
        "<text x=\"30\" y=\"817\" font-size=\"12\">Remaining {} groups: signed {:+.4} pp; magnitude {:.4} pp</text><text x=\"30\" y=\"842\" font-size=\"13\">Baseline {:.4} + contributions {:+.4} = {:.4} · residual {:.2e} · {} evaluations</text><text x=\"30\" y=\"868\" font-size=\"12\">Model-output explanation; not a causal effect or a calibrated cancer probability. Raw values retain their own units.</text></g></svg>",
        rows.len().saturating_sub(10),
        remainder * 100.0,
        abs_remainder * 100.0,
        r.baseline_probability,
        r.attributions.iter().map(|x| x.contribution).sum::<f64>(),
        r.full_probability,
        r.additivity_residual,
        r.evaluations
    )?;
    Ok(out)
}
