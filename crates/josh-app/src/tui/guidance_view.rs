use super::theme::*;
use josh_core::{
    Case,
    guidance::{self, Status},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Paragraph, Row, Table, TableState, Wrap},
};

fn accent(status: Status) -> Color {
    match status {
        Status::Recorded => TEAL,
        Status::Pending => BLUE,
        Status::NeedsReview | Status::InsufficientInformation => AMBER,
        Status::Blocked | Status::Conflict => ROSE,
        Status::ExceptionRecorded => VIOLET,
        _ => MUTED,
    }
}

pub fn draw(
    frame: &mut Frame,
    area: Rect,
    case: &Case,
    selected: &mut u16,
    detail_scroll: &mut u16,
    read_only: bool,
) {
    let Ok(report) = guidance::evaluate(case) else {
        return;
    };
    *selected = (*selected as usize).min(report.items.len().saturating_sub(1)) as u16;
    let [banner, body] = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
    let hint = if read_only {
        "Read-only legacy record · ↑/↓ select · ←/→ scroll · Menu for history"
    } else {
        "↑/↓ select · ←/→ detail scroll · a review · s export · 8 history"
    };
    frame.render_widget(Paragraph::new(format!("NICE CG104 · {} · {}\nClinical signoff pending · selected recommendations · local review only\n{hint}", report.ruleset_version, report.scope))
        .style(Style::default().fg(MUTED)), banner);
    if body.height < 5 {
        return;
    }
    let areas = if area.width >= 105 {
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(body)
    } else {
        Layout::vertical([Constraint::Length(7), Constraint::Min(0)]).split(body)
    };
    let rows = report.items.iter().map(|r| {
        Row::new(vec![r.rule_id.clone(), format!("{:?}", r.status)])
            .style(Style::default().fg(accent(r.status)))
    });
    frame.render_stateful_widget(
        Table::new(
            rows,
            [Constraint::Percentage(52), Constraint::Percentage(48)],
        )
        .header(Row::new(["Rule", "Finding"]).style(Style::default().fg(CYAN)))
        .row_highlight_style(
            Style::default()
                .fg(TEXT)
                .bg(ratatui::style::Color::Rgb(37, 61, 83))
                .bold(),
        )
        .highlight_symbol("› ")
        .block(panel(" Guidance review ", TEAL)),
        areas[0],
        &mut TableState::default().with_selected(Some(*selected as usize)),
    );
    if let Some(item) = report.items.get(*selected as usize) {
        let mut text = format!(
            "NICE {} · {}\n\n{}\n\nRECORDED INPUTS\n{}\n\n{}",
            item.recommendation,
            item.strength,
            item.message,
            item.evidence.join("\n"),
            item.source_url
        );
        if let Some(r) = &item.review {
            text.push_str(&format!(
                "\n\nREVIEW {:?} · {}\n{}\n{}",
                r.action,
                r.assessment.reviewer_id,
                if item.review_is_current {
                    "CURRENT"
                } else {
                    "STALE: case or rules changed"
                },
                r.reason
            ));
        }
        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
        let max_scroll = paragraph
            .line_count(areas[1].width.saturating_sub(2).max(1))
            .saturating_sub(areas[1].height.saturating_sub(2) as usize)
            .min(u16::MAX as usize) as u16;
        *detail_scroll = (*detail_scroll).min(max_scroll);
        frame.render_widget(
            paragraph.scroll((*detail_scroll, 0)).block(panel(
                format!(" {} · {:?} ", item.rule_id, item.status),
                accent(item.status),
            )),
            areas[1],
        );
    }
}
