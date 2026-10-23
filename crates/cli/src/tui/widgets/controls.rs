use crate::tui::state::{Panel, TuiState};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn height(width: u16) -> u16 {
    if width >= 110 {
        5
    } else if width >= 55 {
        7
    } else {
        10
    }
}

fn percentage(used: u64, limit: u64) -> String {
    if limit == 0 {
        "n/a".to_owned()
    } else {
        format!("{:.0}%", used as f64 / limit as f64 * 100.0)
    }
}

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &TuiState) {
    let active = Style::default().fg(Color::Cyan);
    let step = if state.can_step() {
        active
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let cpu = format!(
        "CPU: {} insns ({})",
        state.resources.total_cpu,
        percentage(state.resources.total_cpu, state.resources.cpu_limit)
    );
    let memory = format!(
        "Mem: {} bytes ({})",
        state.resources.total_memory,
        percentage(state.resources.total_memory, state.resources.memory_limit)
    );
    let mut lines = if area.width >= 110 {
        vec![
            Line::from(vec![
                Span::styled("[Tab] Switch Panel  [j/k] Navigate  ", active),
                Span::styled("[s] Step Into  [o] Step Over  ", step),
                Span::styled("[q] Quit", active),
            ]),
            Line::from(format!("{cpu} | {memory}")),
        ]
    } else if area.width >= 55 {
        vec![
            Line::styled("[Tab] Switch Panel  [j/k] Navigate  [q] Quit", active),
            Line::styled("[s] Step Into  [o] Step Over", step),
            Line::from(cpu),
            Line::from(memory),
        ]
    } else {
        vec![
            Line::styled("[Tab] Panel  [q] Quit", active),
            Line::styled("[j/k] Navigate", active),
            Line::styled("[s] Step Into", step),
            Line::styled("[o] Step Over", step),
            Line::from(cpu),
            Line::from(memory),
        ]
    };
    lines.push(Line::from(state.replay_status()));
    let border = if state.selected_panel == Panel::Controls {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Controls | recorded replay totals")
                .border_style(Style::default().fg(border)),
        ),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn footer_renders_controls_status_and_real_percentages() {
        let mut terminal = Terminal::new(TestBackend::new(120, 5)).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), &crate::tui::state::fixture()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        for expected in [
            "[Tab] Switch Panel",
            "[j/k] Navigate",
            "[s] Step Into",
            "[o] Step Over",
            "[q] Quit",
            "250 insns (25%)",
            "500 bytes (50%)",
            "Replay: Paused 1/3",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
    }

    #[test]
    fn missing_limits_are_not_reported_as_zero_and_overruns_are_visible() {
        assert_eq!(percentage(25, 0), "n/a");
        assert_eq!(percentage(0, 100), "0%");
        assert_eq!(percentage(150, 100), "150%");
    }

    #[test]
    fn footer_fits_narrow_terminal_and_survives_tiny_sizes() {
        for (width, rows) in [(60, 7), (40, 10), (1, 1), (0, 0)] {
            let mut terminal = Terminal::new(TestBackend::new(width, rows)).unwrap();
            terminal
                .draw(|frame| render(frame, frame.area(), &crate::tui::state::fixture()))
                .unwrap();
            if width >= 40 {
                let text: String = terminal
                    .backend()
                    .buffer()
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect();
                assert!(text.contains("[q] Quit") && text.contains("Replay: Paused"));
            }
        }
    }
}
