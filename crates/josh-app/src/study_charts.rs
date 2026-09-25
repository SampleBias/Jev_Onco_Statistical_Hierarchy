//! Native cell heatmaps and Braille cohort plots. Every graphic uses computed study results.
use crate::ui;
use josh_core::DataClass;
use josh_features::{
    study::{Report, Study},
    survival::Curve,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::Line,
    widgets::{Axis, Chart, Dataset, GraphType, LegendPosition, Paragraph, Wrap},
};

pub const PALETTE: [Color; 8] = [
    ui::ACCENT,
    ui::BLUE,
    Color::Rgb(202, 154, 255),
    ui::GOLD,
    Color::Rgb(255, 133, 155),
    Color::Rgb(113, 211, 255),
    Color::Rgb(191, 220, 123),
    Color::Rgb(255, 179, 120),
];
pub const HEX: [&str; 8] = [
    "#55e0cc", "#6fa4ff", "#ca9aff", "#f5c668", "#ff859b", "#71d3ff", "#bfdc7b", "#ffb378",
];
#[derive(Clone, Copy, Default)]
pub enum Normalization {
    Count,
    #[default]
    Recall,
    Precision,
}
impl Normalization {
    pub fn next(self) -> Self {
        match self {
            Self::Count => Self::Recall,
            Self::Recall => Self::Precision,
            Self::Precision => Self::Count,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Count => "COUNTS",
            Self::Recall => "ROW % · recall",
            Self::Precision => "COLUMN % · precision",
        }
    }
}
#[derive(Default)]
pub struct PlotState {
    pub row: usize,
    pub column: usize,
    pub normalization: Normalization,
    pub broad: bool,
    pub confidence: bool,
    pub weighted: bool,
    pub curve: usize,
}
pub struct Matrix {
    pub rows: Vec<String>,
    pub columns: Vec<String>,
    pub counts: Vec<Vec<usize>>,
}
impl Matrix {
    pub fn new(data: &Study, report: &Report, broad: bool) -> Option<Self> {
        let m = if broad {
            report.broad_evaluation.as_ref()?
        } else {
            report.evaluation.as_ref()?
        };
        let rows: Vec<_> = m.per_class.keys().cloned().collect();
        let mut columns = rows.clone();
        columns.extend([
            data.taxonomy.unknown_id.clone(),
            data.taxonomy.other_id.clone(),
            "__failed__".into(),
        ]);
        let counts = rows
            .iter()
            .map(|r| {
                columns
                    .iter()
                    .map(|c| {
                        m.confusion
                            .get(r)
                            .and_then(|row| row.get(c))
                            .copied()
                            .unwrap_or(0)
                    })
                    .collect()
            })
            .collect();
        Some(Self {
            rows,
            columns,
            counts,
        })
    }
    pub fn value(&self, r: usize, c: usize, norm: Normalization) -> f64 {
        let count = self.counts[r][c] as f64;
        let denominator = match norm {
            Normalization::Count => 1.,
            Normalization::Recall => self.counts[r].iter().sum::<usize>() as f64,
            Normalization::Precision => self.counts.iter().map(|row| row[c]).sum::<usize>() as f64,
        };
        if denominator > 0. {
            count / denominator
        } else {
            0.
        }
    }
}
fn short(s: &str, width: usize) -> String {
    s.chars().take(width).collect()
}
fn cell(frame: &mut Frame, area: Rect, text: impl Into<String>, style: Style) {
    if area.width > 0 && area.height > 0 {
        frame.render_widget(Paragraph::new(text.into()).style(style), area);
    }
}
pub fn panel(frame: &mut Frame, area: Rect, title: &str) -> Rect {
    let b = ui::panel(title.to_owned());
    let inner = b.inner(area);
    frame.render_widget(b, area);
    inner
}
pub fn empty(frame: &mut Frame, area: Rect, title: &str, message: &str) {
    let inner = panel(frame, area, title);
    frame.render_widget(
        Paragraph::new(message)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(ui::MUTED)),
        inner,
    );
}
fn mix(a: u8, b: u8, v: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * v.clamp(0., 1.)).round() as u8
}
pub fn heat(value: f64, diagonal: bool) -> Color {
    let v = value.clamp(0., 1.).sqrt();
    let end = if diagonal {
        (70, 211, 191)
    } else {
        (192, 114, 196)
    };
    Color::Rgb(mix(20, end.0, v), mix(32, end.1, v), mix(49, end.2, v))
}
pub fn metrics(frame: &mut Frame, area: Rect, data: &Study, report: &Report) {
    let a = report.evaluation.as_ref();
    let cards = [
        (
            " COHORT ",
            a.map(|m| m.eligible.to_string())
                .unwrap_or_else(|| "—".into()),
            a.map(|m| format!("{} returned · {} failed", m.predicted, m.failed))
                .unwrap_or_else(|| "No labeled predictions".into()),
        ),
        (
            " WEIGHTED F1 ",
            a.map(|m| format!("{:.3}", m.weighted_f1))
                .unwrap_or_else(|| "—".into()),
            report
                .bootstrap
                .as_ref()
                .map(|b| {
                    format!(
                        "95% CI {:.3}–{:.3}",
                        b.weighted_f1.lower, b.weighted_f1.upper
                    )
                })
                .unwrap_or_else(|| "Confidence interval unavailable".into()),
        ),
        (
            " COVERAGE @ 0.90 ",
            report
                .thresholds
                .iter()
                .find(|t| t.threshold == 0.9)
                .map(|t| format!("{:.1}%", 100. * t.coverage))
                .unwrap_or_else(|| "—".into()),
            "Biological winners / all eligible".into(),
        ),
        (
            " OUTCOMES ",
            report
                .survival
                .as_ref()
                .map(|s| s.patients.to_string())
                .unwrap_or_else(|| "—".into()),
            report
                .survival
                .as_ref()
                .map(|s| format!("{} events · {} censored", s.events, s.patients - s.events))
                .unwrap_or_else(|| "No outcome records".into()),
        ),
    ];
    let parts = Layout::horizontal([Constraint::Ratio(1, 4); 4]).split(area);
    for (i, ((title, value, detail), rect)) in cards.iter().zip(parts.iter()).enumerate() {
        let inner = panel(frame, *rect, title);
        let label = if data.data_class == DataClass::Synthetic {
            format!("{value}  DEMO")
        } else {
            value.clone()
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(label, Style::default().fg(PALETTE[i]).bold()),
                Line::styled(detail.clone(), Style::default().fg(ui::MUTED)),
            ]),
            inner,
        );
    }
}
pub fn heatmap(frame: &mut Frame, area: Rect, data: &Study, report: &Report, state: &PlotState) {
    let Some(matrix) = Matrix::new(data, report, state.broad) else {
        empty(
            frame,
            area,
            " A / CLASSIFICATION ",
            "Load labeled frozen predictions to inspect the confusion matrix.",
        );
        return;
    };
    let inner = panel(
        frame,
        area,
        &format!(
            " A / {}CONFUSION · {} ",
            if state.broad { "BROAD " } else { "" },
            state.normalization.label()
        ),
    );
    if inner.width < 20 || inner.height < 7 {
        cell(
            frame,
            inner,
            "Enlarge for heatmap; 2 focuses this view.",
            Style::default().fg(ui::MUTED),
        );
        return;
    }
    let label_width = if inner.width > 65 { 10 } else { 7 };
    let available = inner.width.saturating_sub(label_width + 1);
    let cw = (available / matrix.columns.len() as u16).clamp(2, 5);
    let cols = (available / cw) as usize;
    let grid_height = inner.height.saturating_sub(7).max(1);
    let visible_rows = grid_height as usize;
    let drawn_rows = visible_rows.min(matrix.rows.len()) as u16;
    let selected_r = state.row.min(matrix.rows.len() - 1);
    let selected_c = state.column.min(matrix.columns.len() - 1);
    let row_start = selected_r
        .saturating_sub(visible_rows.saturating_sub(1))
        .min(matrix.rows.len().saturating_sub(visible_rows));
    let col_start = selected_c
        .saturating_sub(cols.saturating_sub(1))
        .min(matrix.columns.len().saturating_sub(cols));
    let count_max = matrix
        .counts
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    cell(
        frame,
        Rect::new(inner.x, inner.y, inner.width, 1),
        "TRUTH ↓                 PREDICTED →",
        Style::default().fg(ui::MUTED),
    );
    for (j, name) in matrix.columns.iter().enumerate().skip(col_start).take(cols) {
        let abbreviation = if name == "__failed__" {
            "ERR"
        } else if name == &data.taxonomy.unknown_id {
            "UNK"
        } else if name == &data.taxonomy.other_id {
            "OTH"
        } else {
            name
        };
        cell(
            frame,
            Rect::new(
                inner.x + label_width + (j - col_start) as u16 * cw,
                inner.y + 1 + ((j - col_start) % 2) as u16,
                cw,
                1,
            ),
            short(abbreviation, cw as usize),
            Style::default().fg(if j == selected_c { ui::GOLD } else { ui::MUTED }),
        );
    }
    for (i, name) in matrix
        .rows
        .iter()
        .enumerate()
        .skip(row_start)
        .take(visible_rows)
    {
        let offset = (i - row_start) as u16;
        let y = inner.y + 3 + offset * grid_height / drawn_rows;
        let row_height =
            (offset + 1) * grid_height / drawn_rows - offset * grid_height / drawn_rows;
        cell(
            frame,
            Rect::new(inner.x, y, label_width, 1),
            short(name, label_width as usize),
            Style::default().fg(if i == selected_r { ui::GOLD } else { ui::TEXT }),
        );
        for j in col_start..(col_start + cols).min(matrix.columns.len()) {
            let v = matrix.value(i, j, state.normalization);
            let scale = if matches!(state.normalization, Normalization::Count) {
                v / count_max
            } else {
                v
            };
            let selected = i == selected_r && j == selected_c;
            let text = if matrix.counts[i][j] == 0 {
                "·".into()
            } else if cw < 3 {
                " ".into()
            } else if matches!(state.normalization, Normalization::Count) {
                matrix.counts[i][j].to_string()
            } else {
                format!("{:.0}", 100. * v)
            };
            let text = format!("{text:>width$}", width = cw as usize);
            cell(
                frame,
                Rect::new(
                    inner.x + label_width + (j - col_start) as u16 * cw,
                    y,
                    cw,
                    row_height,
                ),
                text,
                Style::default()
                    .bg(if selected {
                        ui::GOLD
                    } else {
                        heat(scale, i == j)
                    })
                    .fg(if selected || scale > 0.35 {
                        ui::BG
                    } else {
                        ui::MUTED
                    }),
            );
        }
    }
    let recall_y = inner.y + inner.height - 4;
    cell(
        frame,
        Rect::new(inner.x, recall_y, label_width, 1),
        "Recall",
        Style::default().fg(ui::MUTED),
    );
    for j in col_start..(col_start + cols).min(matrix.rows.len()) {
        let value = matrix.value(j, j, Normalization::Recall);
        cell(
            frame,
            Rect::new(
                inner.x + label_width + (j - col_start) as u16 * cw,
                recall_y,
                cw,
                1,
            ),
            format!("{:>width$.0}", 100. * value, width = cw as usize),
            Style::default()
                .bg(heat(value, true))
                .fg(if value > 0.35 { ui::BG } else { ui::TEXT }),
        );
    }
    let count = matrix.counts[selected_r][selected_c];
    let support = matrix.counts[selected_r].iter().sum::<usize>();
    cell(
        frame,
        Rect::new(inner.x, inner.y + inner.height - 3, inner.width, 1),
        format!(
            "{} → {}   {count}/{support} cases",
            matrix.rows[selected_r], matrix.columns[selected_c]
        ),
        Style::default().fg(ui::GOLD),
    );
    cell(
        frame,
        Rect::new(inner.x, inner.y + inner.height - 2, inner.width, 1),
        format!(
            "Rows {}–{}/{} · cols {}–{}/{}",
            row_start + 1,
            (row_start + visible_rows).min(matrix.rows.len()),
            matrix.rows.len(),
            col_start + 1,
            (col_start + cols).min(matrix.columns.len()),
            matrix.columns.len()
        ),
        Style::default().fg(ui::MUTED),
    );
    cell(
        frame,
        Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        "Arrows inspect · n normalize · b broad groups",
        Style::default().fg(ui::MUTED),
    );
}
pub fn steps(
    curve: &Curve,
    field: impl Fn(&josh_features::survival::Point) -> Option<f64>,
) -> Vec<(f64, f64)> {
    let mut result = vec![(curve.start, 1.)];
    let mut previous = 1.;
    for p in &curve.points {
        if let Some(value) = field(p) {
            result.push((p.time, previous));
            result.push((p.time, value));
            previous = value;
        }
    }
    result
}
pub fn survival(
    frame: &mut Frame,
    area: Rect,
    data: &Study,
    report: &Report,
    state: &PlotState,
    treatment: bool,
) {
    let title = if treatment {
        if state.weighted {
            " C / TREATMENT · IPTW "
        } else {
            " C / TREATMENT · UNADJUSTED "
        }
    } else {
        " B / SURVIVAL BY PREDICTED TYPE "
    };
    let (Some(s), Some(input)) = (&report.survival, &data.outcomes) else {
        empty(
            frame,
            area,
            title,
            "No outcome cohort loaded. Add event, follow-up and entry times to plot survival.",
        );
        return;
    };
    let curves = if treatment {
        if state.weighted {
            &s.weighted_concordance
        } else {
            &s.by_concordance
        }
    } else {
        &s.by_class
    };
    if curves.is_empty() {
        empty(
            frame,
            area,
            title,
            s.weighted_unavailable
                .as_deref()
                .unwrap_or("No reviewed concordance groups available."),
        );
        return;
    }
    let inner = panel(frame, area, title);
    if inner.height < 10 || inner.width < 32 {
        cell(
            frame,
            inner,
            if treatment {
                "4 opens the treatment view"
            } else {
                "3 opens the survival view"
            },
            Style::default().fg(ui::MUTED),
        );
        return;
    }
    let page = if treatment { 0 } else { (state.curve / 6) * 6 };
    let shown: Vec<_> = curves.iter().enumerate().skip(page).take(6).collect();
    let risk_height = (shown.len() + 2) as u16;
    let [plot, risk, note] = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(risk_height),
        Constraint::Length(2),
    ])
    .areas(inner);
    let max = input.observations.iter().map(|r| r.time).fold(1., f64::max);
    let lines: Vec<_> = shown
        .iter()
        .map(|(_, c)| steps(c, |p| Some(p.survival)))
        .collect();
    let lower: Vec<_> = shown.iter().map(|(_, c)| steps(c, |p| p.lower)).collect();
    let upper: Vec<_> = shown.iter().map(|(_, c)| steps(c, |p| p.upper)).collect();
    let censors: Vec<Vec<_>> = shown
        .iter()
        .map(|(_, c)| {
            c.points
                .iter()
                .filter(|p| p.censored > 0)
                .map(|p| (p.time, p.survival))
                .collect()
        })
        .collect();
    let mut datasets = Vec::new();
    for (i, (index, c)) in shown.iter().enumerate() {
        let color = PALETTE[*index % PALETTE.len()];
        if state.confidence && !c.weighted {
            datasets.push(
                Dataset::default()
                    .data(&lower[i])
                    .graph_type(GraphType::Line)
                    .marker(Marker::Braille)
                    .style(Style::default().fg(color).dim()),
            );
            datasets.push(
                Dataset::default()
                    .data(&upper[i])
                    .graph_type(GraphType::Line)
                    .marker(Marker::Braille)
                    .style(Style::default().fg(color).dim()),
            );
        }
        datasets.push(
            Dataset::default()
                .name(format!("{} n={}", c.label, c.patients))
                .data(&lines[i])
                .graph_type(GraphType::Line)
                .marker(Marker::Braille)
                .style(Style::default().fg(color)),
        );
        datasets.push(
            Dataset::default()
                .data(&censors[i])
                .graph_type(GraphType::Scatter)
                .marker(Marker::Dot)
                .style(Style::default().fg(color)),
        );
    }
    frame.render_widget(
        Chart::new(datasets)
            .style(Style::default().bg(ui::PANEL))
            .x_axis(
                Axis::default()
                    .bounds([0., max])
                    .labels(
                        (0..=4)
                            .map(|i| Line::from(format!("{:.0}", max * i as f64 / 4.)))
                            .collect::<Vec<_>>(),
                    )
                    .style(Style::default().fg(ui::MUTED)),
            )
            .y_axis(
                Axis::default()
                    .bounds([0., 1.])
                    .labels(["0", ".25", ".50", ".75", "1.0"])
                    .style(Style::default().fg(ui::MUTED)),
            )
            .legend_position(Some(LegendPosition::TopRight))
            .hidden_legend_constraints((Constraint::Percentage(80), Constraint::Percentage(60))),
        plot,
    );
    let label_width = 12_u16.min(risk.width / 3);
    let w = risk.width.saturating_sub(label_width) / 5;
    cell(
        frame,
        Rect::new(risk.x, risk.y, label_width, 1),
        "AT RISK / t",
        Style::default().fg(ui::MUTED),
    );
    for j in 0..5 {
        cell(
            frame,
            Rect::new(risk.x + label_width + j * w, risk.y, w, 1),
            format!("{:>5.1}", max * j as f64 / 4.0),
            Style::default().fg(ui::MUTED),
        );
    }
    for (row, (index, c)) in shown.iter().enumerate() {
        let y = risk.y + 1 + row as u16;
        cell(
            frame,
            Rect::new(risk.x, y, label_width, 1),
            short(&c.label, label_width as usize),
            Style::default().fg(PALETTE[*index % PALETTE.len()]),
        );
        for (j, (_, n)) in c.risk_table.iter().enumerate().take(5) {
            cell(
                frame,
                Rect::new(risk.x + label_width + j as u16 * w, y, w, 1),
                format!("{n:>4}"),
                Style::default().fg(ui::TEXT),
            );
        }
    }
    cell(
        frame,
        Rect::new(
            risk.x,
            risk.y + risk.height.saturating_sub(1),
            risk.width,
            1,
        ),
        format!("{} from {}", input.time_unit, short(&input.time_origin, 55)),
        Style::default().fg(ui::MUTED),
    );
    let message = if treatment && state.weighted {
        "IPTW descriptive curves · raw risk counts · no weighted CI/p".into()
    } else if treatment {
        s.log_rank
            .as_ref()
            .map(|l| {
                format!(
                    "Unweighted log-rank p={:.3e} · {} excluded",
                    l.p_value, s.excluded_from_concordance
                )
            })
            .unwrap_or_else(|| "Unweighted log-rank unavailable".into())
    } else {
        s.subtype_log_rank
            .as_ref()
            .map(|l| {
                format!(
                    "Global log-rank p={:.3e} · df {} · dots: censored",
                    l.p_value, l.degrees_of_freedom
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "{} events · global log-rank unavailable · dots: censored",
                    s.events
                )
            })
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(message, Style::default().fg(ui::MUTED)),
            Line::styled(
                if treatment && state.weighted {
                    "Supplied propensity scores · w shows unadjusted curves"
                } else if state.confidence {
                    "Pointwise 95% CI · i toggle · [/] browse groups"
                } else {
                    "i shows 95% CI · [/] browse groups · w toggles IPTW"
                },
                Style::default().fg(ui::MUTED),
            ),
        ]),
        note,
    );
}
pub fn calibration(frame: &mut Frame, area: Rect, report: &Report) {
    let Some(m) = &report.evaluation else {
        empty(
            frame,
            area,
            " CALIBRATION ",
            "Load labeled predictions to inspect reliability and coverage.",
        );
        return;
    };
    let parts = if area.width >= 100 {
        Layout::horizontal([Constraint::Ratio(1, 2); 2]).split(area)
    } else {
        Layout::vertical([Constraint::Ratio(1, 2); 2]).split(area)
    };
    let reliability: Vec<_> = m
        .reliability
        .iter()
        .filter_map(|b| Some((b.mean_confidence?, b.accuracy?)))
        .collect();
    let ideal = [(0., 0.), (1., 1.)];
    let inner = panel(frame, parts[0], " D / RELIABILITY · 10 BINS ");
    let [chart, note] = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).areas(inner);
    frame.render_widget(
        Chart::new(vec![
            Dataset::default()
                .name("Ideal")
                .data(&ideal)
                .graph_type(GraphType::Line)
                .marker(Marker::Braille)
                .style(Style::default().fg(ui::MUTED)),
            Dataset::default()
                .name("Observed")
                .data(&reliability)
                .graph_type(GraphType::Scatter)
                .marker(Marker::Dot)
                .style(Style::default().fg(ui::ACCENT)),
        ])
        .x_axis(
            Axis::default()
                .title("Mean raw confidence")
                .bounds([0., 1.])
                .labels(["0", ".5", "1"]),
        )
        .y_axis(
            Axis::default()
                .title("Accuracy")
                .bounds([0., 1.])
                .labels(["0", ".5", "1"]),
        )
        .style(Style::default().bg(ui::PANEL)),
        chart,
    );
    frame.render_widget(
        Paragraph::new(format!(
            "ECE {} · Brier {}\nReturned predictions only; no fitted calibration.",
            m.ece
                .map(|x| format!("{x:.3}"))
                .unwrap_or_else(|| "N/A".into()),
            m.brier
                .map(|x| format!("{x:.3}"))
                .unwrap_or_else(|| "N/A".into())
        ))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(ui::MUTED)),
        note,
    );
    let inner = panel(frame, parts[1], " E / COVERAGE vs ACCURACY ");
    let [chart, table] = Layout::vertical([Constraint::Min(3), Constraint::Length(9)]).areas(inner);
    let points: Vec<_> = report
        .thresholds
        .iter()
        .filter_map(|t| Some((t.coverage, t.accuracy?)))
        .collect();
    frame.render_widget(
        Chart::new(vec![
            Dataset::default()
                .data(&points)
                .graph_type(GraphType::Line)
                .marker(Marker::Braille)
                .style(Style::default().fg(ui::BLUE)),
            Dataset::default()
                .data(&points)
                .graph_type(GraphType::Scatter)
                .marker(Marker::Dot)
                .style(Style::default().fg(ui::GOLD)),
        ])
        .x_axis(
            Axis::default()
                .title("Fraction of all eligible cases")
                .bounds([0., 1.])
                .labels(["0", ".5", "1"]),
        )
        .y_axis(
            Axis::default()
                .title("Retained accuracy")
                .bounds([0., 1.])
                .labels(["0", ".5", "1"]),
        )
        .style(Style::default().bg(ui::PANEL)),
        chart,
    );
    let mut lines = vec![Line::styled(
        "p ≥    Retained   Coverage   F1",
        Style::default().fg(ui::MUTED),
    )];
    for t in &report.thresholds {
        lines.push(Line::from(format!(
            "{:.2}   {:>6}    {:>5.1}%   {}",
            t.threshold,
            t.retained,
            100. * t.coverage,
            t.weighted_f1
                .map(|f| format!("{f:.3}"))
                .unwrap_or_else(|| "—".into())
        )));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().fg(ui::TEXT)),
        table,
    );
}
pub fn treatment_details(frame: &mut Frame, area: Rect, report: &Report, scroll: usize) {
    let Some(s) = &report.survival else {
        empty(frame, area, " ADJUSTMENT ", "Outcome data required.");
        return;
    };
    let mut lines = vec![
        Line::styled(
            "TREATMENT ASSOCIATION",
            Style::default().fg(ui::ACCENT).bold(),
        ),
        Line::from("Concordant versus discordant"),
        Line::from(""),
    ];
    if let Some(fit) = &s.cox {
        for c in &fit.coefficients {
            lines.push(Line::styled(c.name.clone(), Style::default().fg(ui::BLUE)));
            lines.push(Line::from(format!(
                "HR {:.3}  95% {:.3}–{:.3}",
                c.hazard_ratio, c.lower, c.upper
            )));
            lines.push(Line::from(format!("p {:.3e}", c.p_value)));
            lines.push(Line::from(""));
        }
        lines.push(Line::from("Cox PH · Breslow ties · Wald CI"));
    } else {
        lines.push(Line::from(s.cox_unavailable.clone().unwrap_or_default()));
    }
    if let Some(w) = &s.weights {
        lines.extend([
            Line::from(""),
            Line::styled("WEIGHT DIAGNOSTICS", Style::default().fg(ui::GOLD)),
            Line::from(format!(
                "Propensity {:.3}–{:.3}",
                w.min_propensity, w.max_propensity
            )),
            Line::from(format!("Maximum weight {:.2}", w.max_weight)),
            Line::from(format!(
                "Effective N {:.1} / {:.1}",
                w.concordant_effective_n, w.discordant_effective_n
            )),
            Line::from(""),
        ]);
        for b in &w.balance {
            lines.push(Line::from(format!(
                "{}: SMD {} → {}",
                b.covariate,
                b.unweighted_smd
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "N/A".into()),
                b.weighted_smd
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "N/A".into())
            )));
        }
    }
    lines.extend([Line::from(""),Line::styled("Association is not treatment benefit.",Style::default().fg(ui::GOLD)),Line::from("PH and confounding assumptions require review. Weighted CI and weighted log-rank are not estimated.")]);
    let inner = panel(frame, area, " MODEL / DIAGNOSTICS ");
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((scroll.min(u16::MAX as usize) as u16, 0))
            .style(Style::default().fg(ui::TEXT)),
        inner,
    );
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn svg_text(out: &mut String, x: f64, y: f64, size: usize, color: &str, text: &str) {
    out.push_str(&format!(
        "<text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{size}\" fill=\"{color}\">{}</text>",
        xml(text)
    ));
}
fn svg_curve(out: &mut String, c: &Curve, rect: [f64; 4], max: f64, color: &str, bands: bool) {
    let [x, y, w, h] = rect;
    let coords = |v: Vec<(f64, f64)>| {
        v.iter()
            .map(|(t, s)| format!("{:.2},{:.2}", x + w * t / max, y + h * (1. - s)))
            .collect::<Vec<_>>()
            .join(" ")
    };
    if bands && !c.weighted {
        let mut polygon = steps(c, |p| p.upper);
        let mut lower = steps(c, |p| p.lower);
        lower.reverse();
        polygon.extend(lower);
        out.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{color}\" opacity=\"0.09\"/>",
            coords(polygon)
        ));
    }
    out.push_str(&format!(
        "<polyline points=\"{}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.4\"/>",
        coords(steps(c, |p| Some(p.survival)))
    ));
    for p in c.points.iter().filter(|p| p.censored > 0) {
        let cx = x + w * p.time / max;
        let cy = y + h * (1. - p.survival);
        out.push_str(&format!(
            "<path d=\"M{:.2},{cy:.2}h6 M{cx:.2},{:.2}v6\" stroke=\"{color}\" stroke-width=\"1\"/>",
            cx - 3.,
            cy - 3.
        ));
    }
}
fn svg_survival(out: &mut String, curves: &[Curve], rect: [f64; 4], title: &str, unit: &str) {
    let [x, y, w, h] = rect;
    svg_text(out, x, y, 16, "#d8e5f1", title);
    if curves.is_empty() {
        svg_text(
            out,
            x,
            y + 35.,
            13,
            "#859bb0",
            "Outcome analysis unavailable",
        );
        return;
    }
    let max = curves
        .iter()
        .flat_map(|c| c.risk_table.iter().map(|(t, _)| *t))
        .fold(1., f64::max);
    let top = y + 24.;
    let plot_h = h - 110.;
    let left = x + 35.;
    let plot_w = w - 55.;
    for i in 0..=4 {
        let s = i as f64 / 4.;
        let yy = top + plot_h * (1. - s);
        out.push_str(&format!(
            "<path d=\"M{left},{yy}h{plot_w}\" stroke=\"#233448\"/>"
        ));
        svg_text(out, x, yy + 4., 11, "#859bb0", &format!("{s:.2}"));
        let xx = left + plot_w * s;
        svg_text(
            out,
            xx - 5.,
            top + plot_h + 20.,
            11,
            "#859bb0",
            &format!("{:.0}", max * s),
        );
    }
    for (i, c) in curves.iter().enumerate() {
        svg_curve(
            out,
            c,
            [left, top, plot_w, plot_h],
            max,
            HEX[i % 8],
            curves.len() <= 2,
        );
    }
    svg_text(
        out,
        left,
        top + plot_h + 40.,
        12,
        "#859bb0",
        &format!(
            "Follow-up ({unit}) · + censored{}",
            if curves.len() <= 2 {
                " · shaded: pointwise 95% CI"
            } else {
                ""
            }
        ),
    );
    for (i, c) in curves.iter().enumerate().take(6) {
        svg_text(
            out,
            left + (i % 2) as f64 * (plot_w / 2.),
            top + plot_h + 60. + (i / 2) as f64 * 17.,
            11,
            HEX[i % 8],
            &format!(
                "{} n={} / events={}",
                short(&c.label, 24),
                c.patients,
                c.events
            ),
        );
    }
    if curves.len() > 6 {
        svg_text(
            out,
            left,
            y + h + 8.,
            11,
            "#859bb0",
            &format!(
                "All {} groups plotted; see JSON for full labels",
                curves.len()
            ),
        );
    }
}
fn svg_risk(out: &mut String, curves: &[Curve], x: f64, y: f64, title: &str) {
    svg_text(out, x, y, 14, "#d8e5f1", title);
    if let Some(first) = curves.first() {
        for (j, (t, _)) in first.risk_table.iter().enumerate() {
            svg_text(
                out,
                x + 200. + j as f64 * 90.,
                y + 24.,
                11,
                "#859bb0",
                &format!("t={t:.1}"),
            );
        }
    }
    for (i, c) in curves.iter().enumerate().take(6) {
        svg_text(
            out,
            x,
            y + 46. + i as f64 * 19.,
            11,
            HEX[i % 8],
            &short(&c.label, 25),
        );
        for (j, (_, n)) in c.risk_table.iter().enumerate() {
            svg_text(
                out,
                x + 200. + j as f64 * 90.,
                y + 46. + i as f64 * 19.,
                11,
                "#d8e5f1",
                &n.to_string(),
            );
        }
    }
}
pub fn svg(data: &Study, report: &Report) -> String {
    let mut out = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1600\" height=\"1450\" viewBox=\"0 0 1600 1450\"><rect width=\"1600\" height=\"1450\" fill=\"#0a111c\"/><g font-family=\"DejaVu Sans, sans-serif\">",
    );
    svg_text(
        &mut out,
        42.,
        46.,
        13,
        "#55e0cc",
        "COHORT OBSERVATORY / JEV RESEARCH WORKSPACE",
    );
    svg_text(&mut out, 42., 88., 30, "#d8e5f1", &short(&data.title, 80));
    svg_text(
        &mut out,
        42.,
        118.,
        14,
        "#f5c668",
        if data.data_class == DataClass::Synthetic {
            "SYNTHETIC DEMONSTRATION — invented data; not Jev or OncoNPC performance"
        } else {
            "Imported frozen predictions · observational outcomes · research analysis"
        },
    );
    let meta = report
        .evaluation
        .as_ref()
        .map(|m| {
            format!(
                "{} eligible   |   weighted F1 {:.3}   |   {} failed   |   partition {}",
                m.eligible, m.weighted_f1, m.failed, data.partition
            )
        })
        .unwrap_or_else(|| "Outcome cohort only".into());
    svg_text(&mut out, 42., 150., 14, "#859bb0", &meta);
    if let Some(matrix) = Matrix::new(data, report, false) {
        svg_text(
            &mut out,
            42.,
            195.,
            16,
            "#d8e5f1",
            "A / Classification · row-normalized confusion (%)",
        );
        let cw = (640. / matrix.columns.len() as f64).min(24.);
        let ch = (610. / matrix.rows.len() as f64).min(26.);
        let x = 135.;
        let y = 235.;
        for (j, name) in matrix.columns.iter().enumerate() {
            let abbreviation = if name == "__failed__" {
                "ERR"
            } else if name == &data.taxonomy.unknown_id {
                "UNK"
            } else if name == &data.taxonomy.other_id {
                "OTH"
            } else {
                name
            };
            out.push_str(&format!("<text x=\"{}\" y=\"{}\" font-size=\"9\" fill=\"#859bb0\" transform=\"rotate(-50 {} {})\">{}</text>",x+j as f64*cw+8.,y-8.,x+j as f64*cw+8.,y-8.,xml(&short(abbreviation,7))));
        }
        for (i, name) in matrix.rows.iter().enumerate() {
            svg_text(
                &mut out,
                42.,
                y + i as f64 * ch + ch * 0.7,
                11,
                "#d8e5f1",
                &short(name, 12),
            );
            for j in 0..matrix.columns.len() {
                let v = matrix.value(i, j, Normalization::Recall);
                let Color::Rgb(r, g, b) = heat(v, i == j) else {
                    unreachable!()
                };
                let xx = x + j as f64 * cw;
                let yy = y + i as f64 * ch;
                out.push_str(&format!("<rect x=\"{xx}\" y=\"{yy}\" width=\"{}\" height=\"{}\" fill=\"#{r:02x}{g:02x}{b:02x}\"/>",cw-1.,ch-1.));
                if matrix.counts[i][j] > 0 {
                    svg_text(
                        &mut out,
                        xx + 4.,
                        yy + ch * 0.7,
                        9,
                        if v > 0.35 { "#0a111c" } else { "#d8e5f1" },
                        &format!("{:.0}", v * 100.),
                    );
                }
            }
        }
        let bottom = y + matrix.rows.len() as f64 * ch;
        svg_text(&mut out, 42., bottom + 23., 11, "#859bb0", "Recall %");
        for j in 0..matrix.rows.len() {
            let v = matrix.value(j, j, Normalization::Recall);
            let Color::Rgb(r, g, b) = heat(v, true) else {
                unreachable!()
            };
            let xx = x + j as f64 * cw;
            out.push_str(&format!("<rect x=\"{xx}\" y=\"{}\" width=\"{}\" height=\"19\" fill=\"#{r:02x}{g:02x}{b:02x}\"/>",bottom+9.,cw-1.));
            svg_text(
                &mut out,
                xx + 3.,
                bottom + 23.,
                9,
                if v > 0.35 { "#0a111c" } else { "#d8e5f1" },
                &format!("{:.0}", v * 100.),
            );
        }
        svg_text(
            &mut out,
            42.,
            bottom + 50.,
            12,
            "#859bb0",
            "Truth: rows · prediction: columns · unknown/other/failures retained",
        );
        if let Some(b) = &report.bootstrap {
            svg_text(
                &mut out,
                42.,
                bottom + 75.,
                13,
                "#55e0cc",
                &format!(
                    "Weighted F1 95% CI {:.3}–{:.3} · {} patient bootstrap replicates",
                    b.weighted_f1.lower, b.weighted_f1.upper, b.replicates
                ),
            );
        }
    } else {
        svg_text(
            &mut out,
            42.,
            215.,
            14,
            "#859bb0",
            "No labeled classification records",
        );
    }
    if let (Some(s), Some(input)) = (&report.survival, &data.outcomes) {
        svg_survival(
            &mut out,
            &s.by_class,
            [815., 195., 735., 385.],
            "B / Survival by predicted cancer type",
            &input.time_unit,
        );
        svg_survival(
            &mut out,
            &s.by_concordance,
            [815., 615., 735., 365.],
            "C / Treatment concordance · unadjusted",
            &input.time_unit,
        );
        if let Some(lr) = &s.subtype_log_rank {
            svg_text(
                &mut out,
                850.,
                598.,
                11,
                "#859bb0",
                &format!(
                    "Global log-rank p={:.3e} · df {}",
                    lr.p_value, lr.degrees_of_freedom
                ),
            );
        }
        svg_text(
            &mut out,
            42.,
            1210.,
            12,
            "#859bb0",
            &format!(
                "Endpoint: {} · {} from {}",
                short(&input.endpoint, 45),
                input.time_unit,
                short(&input.time_origin, 65)
            ),
        );
        svg_risk(
            &mut out,
            &s.by_class,
            42.,
            1240.,
            "B / NUMBER AT RISK (first six groups; full table in JSON)",
        );
        svg_risk(
            &mut out,
            &s.by_concordance,
            815.,
            1240.,
            "C / NUMBER AT RISK · unweighted patients",
        );
        if let Some(lr) = &s.log_rank {
            svg_text(
                &mut out,
                850.,
                1005.,
                12,
                "#859bb0",
                &format!(
                    "Unweighted log-rank p={:.3e}; {} excluded (empiric/unreviewed)",
                    lr.p_value, s.excluded_from_concordance
                ),
            );
        }
        if let Some(cox) = &s.cox {
            let c = &cox.coefficients[0];
            svg_text(
                &mut out,
                850.,
                1030.,
                13,
                "#55e0cc",
                &format!(
                    "Adjusted Cox HR {:.3} [95% {:.3}–{:.3}]",
                    c.hazard_ratio, c.lower, c.upper
                ),
            );
        }
    } else {
        svg_text(
            &mut out,
            840.,
            215.,
            14,
            "#859bb0",
            "No outcome cohort supplied",
        );
    }
    if !report.thresholds.is_empty() {
        svg_text(&mut out, 42., 935., 15, "#d8e5f1", "CONFIDENCE / COVERAGE");
        for (i, t) in report.thresholds.iter().enumerate() {
            let y = 965. + i as f64 * 20.;
            svg_text(
                &mut out,
                42.,
                y,
                12,
                "#859bb0",
                &format!("p ≥ {:.2}", t.threshold),
            );
            out.push_str(&format!(
                "<rect x=\"116\" y=\"{}\" width=\"{}\" height=\"9\" rx=\"3\" fill=\"#6fa4ff\"/>",
                y - 9.,
                t.coverage * 300.
            ));
            svg_text(
                &mut out,
                435.,
                y,
                12,
                "#d8e5f1",
                &format!(
                    "{:5.1}% retained · F1 {}",
                    100. * t.coverage,
                    t.weighted_f1
                        .map(|v| format!("{v:.3}"))
                        .unwrap_or_else(|| "N/A".into())
                ),
            );
        }
    }
    svg_text(
        &mut out,
        42.,
        1120.,
        11,
        "#859bb0",
        &format!(
            "Study {} · SHA-256 {}",
            short(&data.study_id, 50),
            report.study_sha256
        ),
    );
    svg_text(
        &mut out,
        42.,
        1143.,
        12,
        "#f5c668",
        "Research only. Model scores and retrospective associations do not establish cancer accuracy or treatment benefit.",
    );
    svg_text(
        &mut out,
        42.,
        1166.,
        11,
        "#859bb0",
        "Reference: Moon et al. OncoNPC · doi:10.1101/2022.12.22.22283696 · doi:10.1038/s41591-023-02482-6",
    );
    out.push_str("</g></svg>");
    out
}
