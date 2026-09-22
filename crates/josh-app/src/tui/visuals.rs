use super::theme::*;
use crate::workflows::display_text;
use josh_core::{
    Case, EvidenceKind, ObservationStatus, ResultRecord, Source, categorical_status, clinical::*,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Axis, Bar, BarChart, Cell, Chart, Dataset, GraphType, Paragraph, Row, Table, Tabs, Wrap,
    },
};

pub const VIEW_COUNT: usize = 5;

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    case: &Case,
    result: Option<&ResultRecord>,
    page: usize,
    scroll: &mut u16,
) {
    let [nav, body] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    let names = if area.width >= 80 {
        vec!["Scores", "IHC matrix", "Pathway", "Timeline", "Evidence"]
    } else {
        vec!["Scores", "IHC", "Path", "Time", "Evidence"]
    };
    frame.render_widget(
        Tabs::new(names)
            .select(page)
            .divider("·")
            .style(Style::default().fg(MUTED))
            .highlight_style(Style::default().fg(BACKGROUND).bg(TEAL).bold())
            .block(ratatui::widgets::Block::new().title_bottom("←/→ views · ↑/↓ scroll")),
        nav,
    );
    if body.height < 4 {
        return;
    }
    match page {
        0 => scores(frame, body, result, scroll),
        1 => ihc(frame, body, case, scroll),
        2 => pathway(frame, body, case),
        3 => timeline(frame, body, case, scroll),
        _ => evidence(frame, body, case),
    }
}

fn note(frame: &mut Frame, area: Rect, title: &str, text: &str) {
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(panel(title.to_owned(), MUTED)),
        area,
    );
}

fn scores(frame: &mut Frame, area: Rect, result: Option<&ResultRecord>, scroll: &mut u16) {
    let Some(result) = result else {
        note(
            frame,
            area,
            " Origin scores ",
            "No result yet. Press d for the labelled offline mock, then 6 to return.\nLive synthetic classification uses c and confirmation.\nNo probability bars are invented for missing results.",
        );
        return;
    };
    let badge = match result.source {
        Source::Mock => "MOCK / NO PREDICTION",
        Source::Replay => "REPLAY / UNVERIFIED",
        Source::Jev => "JEV / RESEARCH",
    };
    let [notice, plot] = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                format!("{badge} · {}", result.status),
                Style::default().fg(AMBER).bold(),
            ),
            Line::from("Uncalibrated model scores · fixed 0–100% scale"),
            Line::styled(
                "All outcomes retained; scrolling never rescales scores.",
                Style::default().fg(MUTED),
            ),
        ]),
        notice,
    );
    let count = (plot.height.saturating_sub(2) as usize).max(1);
    *scroll = (*scroll as usize).min(result.rankings.len().saturating_sub(count)) as u16;
    let start = *scroll as usize;
    let block = panel(
        format!(
            " Origins {}–{} / {} ",
            start + 1,
            (start + count).min(result.rankings.len()),
            result.rankings.len()
        ),
        TEAL,
    );
    let inner = block.inner(plot);
    frame.render_widget(block, plot);
    // Separate labels/numbers keep small and zero scores readable at any bar length.
    let [labels, bars] = Layout::horizontal([
        Constraint::Length(32.min(inner.width.saturating_sub(8))),
        Constraint::Min(0),
    ])
    .areas(inner);
    let visible: Vec<_> = result.rankings.iter().skip(start).take(count).collect();
    let lines: Vec<_> = visible
        .iter()
        .map(|r| {
            Line::from(format!(
                "{:<24} {:>5.1}%",
                r.origin,
                r.raw_probability * 100.0
            ))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), labels);
    let data: Vec<_> = visible
        .iter()
        .enumerate()
        .map(|(i, r)| {
            Bar::default()
                .value((r.raw_probability * 10000.0).round() as u64)
                .text_value("")
                .style(Style::default().fg(
                    if matches!(r.origin.as_str(), "insufficient_evidence" | "other_origin") {
                        MUTED
                    } else {
                        SERIES[(start + i) % SERIES.len()]
                    },
                ))
        })
        .collect();
    frame.render_widget(
        BarChart::horizontal(data)
            .max(10000)
            .bar_width(1)
            .bar_gap(0),
        bars,
    );
}

fn status(f: &josh_core::Finding) -> ObservationStatus {
    f.observation
        .as_ref()
        .map(|o| o.status)
        .or_else(|| categorical_status(&f.value))
        .unwrap_or(ObservationStatus::Observed)
}
fn status_style(s: ObservationStatus) -> (&'static str, Color) {
    match s {
        ObservationStatus::Positive => ("+ POS", TEAL),
        ObservationStatus::Negative => ("- NEG", BLUE),
        ObservationStatus::Equivocal => ("~ EQV", AMBER),
        ObservationStatus::NotTested => ("NT", MUTED),
        ObservationStatus::Unknown => ("? UNK", MUTED),
        ObservationStatus::Observed => ("OBS", VIOLET),
    }
}

fn ihc(frame: &mut Frame, area: Rect, case: &Case, scroll: &mut u16) {
    let [legend, table] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    frame.render_widget(Paragraph::new("IHC = immunohistochemistry · recorded evidence\n+ positive  - negative  ~ equivocal  NT not tested  ? unknown").style(Style::default().fg(MUTED)), legend);
    let findings: Vec<_> = case
        .findings
        .iter()
        .filter(|f| f.kind == EvidenceKind::Ihc)
        .collect();
    if findings.is_empty() {
        note(
            frame,
            table,
            " IHC evidence matrix ",
            "No IHC findings recorded. Missing tests are unknown.",
        );
        return;
    }
    let count = (table.height.saturating_sub(3) as usize).max(1);
    *scroll = (*scroll as usize).min(findings.len().saturating_sub(count)) as u16;
    let wide = area.width >= 90;
    let rows = findings.iter().skip(*scroll as usize).take(count).map(|f| {
        let (label, color) = status_style(status(f));
        let mut cells = vec![
            Cell::from(display_text(&f.name)),
            Cell::from(label).style(Style::default().fg(BACKGROUND).bg(color).bold()),
            Cell::from(display_text(&f.value)),
        ];
        if wide {
            cells.push(Cell::from(
                f.observation
                    .as_ref()
                    .and_then(|o| o.timepoint.as_deref())
                    .map(display_text)
                    .unwrap_or_else(|| "not dated".into()),
            ));
        }
        Row::new(cells)
    });
    let mut widths = vec![
        Constraint::Percentage(20),
        Constraint::Length(7),
        Constraint::Min(10),
    ];
    let mut headings = vec!["Marker", "State", "Recorded result"];
    if wide {
        widths.push(Constraint::Length(18));
        headings.push("Timepoint");
    }
    frame.render_widget(
        Table::new(rows, widths)
            .header(
                Row::new(headings)
                    .style(Style::default().fg(CYAN))
                    .bottom_margin(0),
            )
            .column_spacing(2)
            .block(panel(
                format!(
                    " IHC {}–{} / {} · source details in Evidence ",
                    *scroll as usize + 1,
                    (*scroll as usize + count).min(findings.len()),
                    findings.len()
                ),
                CYAN,
            )),
        table,
    );
}

fn pathway(frame: &mut Frame, area: Rect, case: &Case) {
    let stage = case.clinical.as_ref().map(|c| c.stage).unwrap_or_default();
    let [heading, flow, detail] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(5),
        Constraint::Min(0),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(format!("Recorded stage: {stage:?}\nClinician assessment only · stages are never inferred from model scores.")).wrap(Wrap {trim:false}), heading);
    let parts = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(flow);
    for (index, (value, label)) in [
        (DiagnosticStage::Muo, "MUO"),
        (DiagnosticStage::ProvisionalCup, "Provisional CUP"),
        (DiagnosticStage::ConfirmedCup, "Confirmed CUP"),
    ]
    .iter()
    .enumerate()
    {
        let selected = stage == *value;
        frame.render_widget(
            Paragraph::new(format!(
                "{label}\n{}",
                if selected { "RECORDED HERE" } else { "—" }
            ))
            .centered()
            .block(panel(
                format!(" {} {} ", index + 1, if index < 2 { "→" } else { "" }),
                if selected { TEAL } else { BLUE },
            ))
            .style(Style::default().fg(if selected { TEAL } else { MUTED })),
            parts[index],
        );
    }
    let text = if let Some(c) = &case.clinical {
        let f = |key| match c.feature(key) {
            Some(true) => "recorded yes",
            Some(false) => "recorded no",
            None => "unknown",
        };
        format!(
            "Lineage: {:?}\nHistology: {}\nInitial workup: {}\nSpecialist review: {}\nFurther appropriate investigations: {}\n\nA primary may be identified or an alternative lineage found at any point. See 7 Guidance for conflicts and source links.",
            c.lineage,
            f(Feature::HistologyConfirmed),
            f(Feature::InitialWorkupComplete),
            f(Feature::SpecialistReviewComplete),
            f(Feature::FurtherInvestigationsComplete)
        )
    } else {
        "No structured clinical assessment in this case.\nUse a schema 3 case to record stage and workup.\n\nNICE CG104 terms are a reference pathway; this diagram is not a diagnosis.".into()
    };
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(panel(" Assessment basis ", VIOLET)),
        detail,
    );
}

fn timeline(frame: &mut Frame, area: Rect, case: &Case, scroll: &mut u16) {
    let Some(context) = &case.clinical else {
        note(
            frame,
            area,
            " Investigation timeline ",
            "No structured investigation dates recorded.\nFinding timepoint labels remain available in Evidence. They are not assumed to be dates.\nUse a schema 3 clinical case for a relative-day timeline.",
        );
        return;
    };
    if context.investigations.is_empty() {
        note(
            frame,
            area,
            " Investigation timeline ",
            "No investigation records. Unknown dates are not placed at day zero.",
        );
        return;
    }
    let [plot, list] =
        Layout::vertical([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(area);
    let mut events: Vec<_> = context.investigations.iter().collect();
    events.sort_by_key(|i| (i.day.is_none(), i.day.unwrap_or(0)));
    let dated: Vec<_> = events
        .iter()
        .filter_map(|i| i.day.map(|d| (d as f64, 1.0)))
        .collect();
    if dated.is_empty() {
        note(
            frame,
            plot,
            " Relative-day timeline ",
            "Dates unknown. Events are listed below without invented chronology.",
        );
    } else {
        let min = dated.first().unwrap().0 - 1.0;
        let max = dated.last().unwrap().0 + 1.0;
        let done: Vec<_> = events
            .iter()
            .filter(|i| i.status == InvestigationStatus::Completed)
            .filter_map(|i| i.day.map(|d| (d as f64, 1.0)))
            .collect();
        let other: Vec<_> = events
            .iter()
            .filter(|i| i.status != InvestigationStatus::Completed)
            .filter_map(|i| i.day.map(|d| (d as f64, 2.0)))
            .collect();
        frame.render_widget(
            Chart::new(vec![
                Dataset::default()
                    .name("Completed")
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Scatter)
                    .style(Style::default().fg(TEAL))
                    .data(&done),
                Dataset::default()
                    .name("Other status")
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Scatter)
                    .style(Style::default().fg(VIOLET))
                    .data(&other),
            ])
            .block(panel(" Relative day · shared case reference ", BLUE))
            .x_axis(
                Axis::default()
                    .bounds([min, max])
                    .labels([format!("{min:.0}"), format!("{max:.0}")])
                    .style(Style::default().fg(MUTED)),
            )
            .y_axis(Axis::default().bounds([0.0, 3.0])),
            plot,
        );
    }
    let count = list.height.saturating_sub(3).max(1) as usize;
    *scroll = (*scroll as usize).min(events.len().saturating_sub(count)) as u16;
    let rows = events.iter().skip(*scroll as usize).take(count).map(|i| {
        Row::new(vec![
            i.day
                .map(|d| format!("{d:+}"))
                .unwrap_or_else(|| "unknown".into()),
            format!("{:?}", i.kind),
            format!("{:?}", i.status),
        ])
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Min(15),
                Constraint::Length(16),
            ],
        )
        .header(Row::new(["Day", "Investigation", "Status"]).style(Style::default().fg(CYAN)))
        .block(panel(
            format!(
                " Events {}–{} / {} ",
                *scroll as usize + 1,
                (*scroll as usize + count).min(events.len()),
                events.len()
            ),
            BLUE,
        )),
        list,
    );
}

fn evidence(frame: &mut Frame, area: Rect, case: &Case) {
    let kinds = [
        EvidenceKind::Histology,
        EvidenceKind::Ihc,
        EvidenceKind::Molecular,
        EvidenceKind::Clinical,
    ];
    let bars = kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            let count = case.findings.iter().filter(|f| f.kind == *kind).count();
            Bar::with_label(format!("{kind:?}"), count as u64)
                .style(Style::default().fg(SERIES[index]))
                .value_style(Style::default().fg(BACKGROUND).bg(SERIES[index]))
        })
        .collect::<Vec<_>>();
    let [chart, caption] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(4)]).areas(area);
    frame.render_widget(
        BarChart::horizontal(bars)
            .bar_width(1)
            .bar_gap(1)
            .label_style(Style::default().fg(TEXT))
            .block(panel(" Recorded findings by type ", VIOLET)),
        chart,
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("{} findings", case.findings.len()),
                    Style::default().fg(TEAL),
                ),
                Span::raw(" · counts include unknown/not-tested records"),
            ]),
            Line::from("Counts measure documentation, not diagnostic adequacy."),
            Line::from(
                "Calibration, confusion matrices and survival plots need validated cohort data.",
            ),
        ])
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(MUTED)),
        caption,
    );
}
