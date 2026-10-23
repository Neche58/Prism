#![allow(dead_code)]

use grat_core::types::trace::ContractInvocation;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Number of columns used to indent a single level of the call hierarchy.
const INDENT_WIDTH: usize = 2;

/// Contract ids longer than this are elided in the middle.
const CONTRACT_ID_MAX_LEN: usize = 16;

/// Number of leading characters kept when a contract id is elided.
const CONTRACT_ID_HEAD_LEN: usize = 8;

/// Number of trailing characters kept when a contract id is elided.
const CONTRACT_ID_TAIL_LEN: usize = 4;

/// Symbol rendered in front of the focused frame.
const FOCUS_SYMBOL: &str = "▶ ";

/// Message rendered when the replay did not record any invocation.
const EMPTY_MESSAGE: &str = "No contract invocations recorded";

/// A single entry of the invocation timeline.
#[derive(Debug, Clone, Copy)]
pub struct TimelineRow<'a> {
    /// Nesting depth of the invocation; `0` is a top level transaction call.
    pub depth: usize,

    /// `true` when the invocation is the last one of its sibling group.
    pub is_last: bool,

    /// The invocation rendered by this row.
    pub invocation: &'a ContractInvocation,
}

/// Counts the invocations of a tree without flattening it.
pub fn count(invocations: &[ContractInvocation]) -> usize {
    invocations
        .iter()
        .map(|invocation| 1 + count(&invocation.sub_invocations))
        .sum()
}

/// Flattens the invocation tree into timeline order.
///
/// Invocations are listed depth first, exactly like the trace tree printed by
/// `grat trace`, so a timeline index always points at the same invocation.
pub fn flatten(invocations: &[ContractInvocation]) -> Vec<TimelineRow<'_>> {
    let mut rows = Vec::new();
    collect_rows(invocations, 0, &mut rows);
    rows
}

fn collect_rows<'a>(
    invocations: &'a [ContractInvocation],
    depth: usize,
    rows: &mut Vec<TimelineRow<'a>>,
) {
    let last_index = invocations.len().saturating_sub(1);
    for (index, invocation) in invocations.iter().enumerate() {
        rows.push(TimelineRow {
            depth,
            is_last: index == last_index,
            invocation,
        });
        collect_rows(&invocation.sub_invocations, depth + 1, rows);
    }
}

/// Renders the invocation timeline of a replayed transaction.
///
/// `current_index` points at the focused frame inside the flattened timeline;
/// that frame is highlighted and scrolled into view.
pub fn render(
    frame: &mut Frame<'_>,
    area: Rect,
    invocations: &[ContractInvocation],
    current_index: usize,
) {
    render_with_focus(frame, area, invocations, current_index, true);
}

/// Renders the invocation timeline of a replayed transaction.
///
/// The panel border is highlighted while the timeline owns the keyboard focus.
pub fn render_with_focus(
    frame: &mut Frame<'_>,
    area: Rect,
    invocations: &[ContractInvocation],
    current_index: usize,
    focused: bool,
) {
    let rows = flatten(invocations);
    let block = panel(focused, current_index, rows.len());

    if rows.is_empty() {
        frame.render_widget(Paragraph::new(EMPTY_MESSAGE).block(block), area);
        return;
    }

    let selected = current_index.min(rows.len() - 1);
    let items: Vec<ListItem<'_>> = rows
        .iter()
        .map(|row| ListItem::new(row_line(row)))
        .collect();
    let mut state = ListState::default().with_selected(Some(selected));

    let list = List::new(items)
        .block(block)
        .highlight_symbol(FOCUS_SYMBOL)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_spacing(HighlightSpacing::Always);

    frame.render_stateful_widget(list, area, &mut state);
}

/// Builds the panel that frames the timeline.
fn panel(focused: bool, current_index: usize, total: usize) -> Block<'static> {
    let position = if total == 0 {
        0
    } else {
        current_index.min(total - 1) + 1
    };

    Block::default()
        .borders(Borders::ALL)
        .title(format!(" Timeline ({position}/{total}) "))
        .border_style(Style::default().fg(if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }))
}

/// Renders a single invocation as a timeline row.
fn row_line(row: &TimelineRow<'_>) -> Line<'static> {
    let invocation = row.invocation;
    let indent = " ".repeat(INDENT_WIDTH * row.depth);
    let connector = if row.is_last { "└─ " } else { "├─ " };

    let (status, status_style) = if invocation.is_error {
        (
            "✘ ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else {
        ("✔ ", Style::default().fg(Color::Green))
    };

    Line::from(vec![
        Span::styled(
            format!("{indent}{connector}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(status, status_style),
        Span::styled(
            invocation.function_name.clone(),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            shorten_contract_id(&invocation.contract_id),
            Style::default().fg(Color::Cyan),
        ),
    ])
}

/// Shortens a contract id so that it fits next to the call hierarchy.
///
/// Stellar contract ids are 56 characters long, which does not fit in a
/// terminal panel, so long ids are elided in the middle while keeping their
/// prefix and suffix recognisable.
fn shorten_contract_id(contract_id: &str) -> String {
    let length = contract_id.chars().count();
    if length <= CONTRACT_ID_MAX_LEN {
        return contract_id.to_string();
    }

    let head: String = contract_id.chars().take(CONTRACT_ID_HEAD_LEN).collect();
    let tail: String = contract_id
        .chars()
        .skip(length - CONTRACT_ID_TAIL_LEN)
        .collect();

    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use grat_core::types::trace::{HostFunctionCall, TraceId};
    use ratatui::{backend::TestBackend, Terminal};

    fn invocation(contract_id: &str, function_name: &str, is_error: bool) -> ContractInvocation {
        ContractInvocation {
            trace_id: TraceId::new(),
            contract_id: contract_id.to_string(),
            function_name: function_name.to_string(),
            arguments: Vec::new(),
            return_value: None,
            host_calls: Vec::<HostFunctionCall>::new(),
            sub_invocations: Vec::new(),
            total_cpu_instructions: 0,
            total_memory_bytes: 0,
            is_error,
        }
    }

    fn sample_tree() -> Vec<ContractInvocation> {
        let mut transfer = invocation("CONTRACT_A", "transfer", false);
        transfer.sub_invocations = vec![invocation("CONTRACT_B", "mint", false)];

        vec![
            transfer,
            invocation("CONTRACT_C", "swap", true),
            invocation("CONTRACT_D", "deposit", false),
        ]
    }

    fn render_to_buffer(
        invocations: &[ContractInvocation],
        current_index: usize,
        width: u16,
        height: u16,
    ) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), invocations, current_index))
            .unwrap();
        terminal
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn flatten_lists_invocations_depth_first() {
        let invocations = sample_tree();
        let rows = flatten(&invocations);

        let names: Vec<&str> = rows
            .iter()
            .map(|row| row.invocation.function_name.as_str())
            .collect();
        assert_eq!(names, vec!["transfer", "mint", "swap", "deposit"]);

        let depths: Vec<usize> = rows.iter().map(|row| row.depth).collect();
        assert_eq!(depths, vec![0, 1, 0, 0]);
    }

    #[test]
    fn flatten_marks_the_last_sibling_of_every_group() {
        let invocations = sample_tree();
        let rows = flatten(&invocations);

        let last_flags: Vec<bool> = rows.iter().map(|row| row.is_last).collect();
        assert_eq!(last_flags, vec![false, true, false, true]);
    }

    #[test]
    fn count_matches_the_number_of_flattened_rows() {
        let invocations = sample_tree();

        assert_eq!(count(&invocations), 4);
        assert_eq!(count(&invocations), flatten(&invocations).len());
        assert_eq!(count(&[]), 0);
    }

    #[test]
    fn render_shows_hierarchy_functions_contracts_and_status() {
        let invocations = sample_tree();
        let terminal = render_to_buffer(&invocations, 0, 60, 10);
        let text = buffer_text(&terminal);

        assert!(text.contains("transfer"));
        assert!(text.contains("mint"));
        assert!(text.contains("swap"));
        assert!(text.contains("CONTRACT_A"));
        assert!(text.contains("CONTRACT_B"));
        assert!(text.contains("✔"));
        assert!(text.contains("✘"));
    }

    #[test]
    fn render_indents_nested_invocations() {
        let invocations = sample_tree();
        let terminal = render_to_buffer(&invocations, 0, 60, 10);
        let buffer = terminal.backend().buffer();

        // Rows start after the panel border and the focus symbol.
        let top_level = buffer.cell((3, 1)).unwrap().symbol().to_string();
        let nested_indent = buffer.cell((3, 2)).unwrap().symbol().to_string();
        let nested_connector = buffer.cell((5, 2)).unwrap().symbol().to_string();

        assert_eq!(top_level, "├");
        assert_eq!(nested_indent, " ");
        assert_eq!(nested_connector, "└");
    }

    #[test]
    fn render_highlights_the_focused_frame() {
        let invocations = sample_tree();
        let terminal = render_to_buffer(&invocations, 2, 60, 10);
        let buffer = terminal.backend().buffer();

        // The focused frame is the third row of the list, its function name
        // starts after the connector and the status marker.
        let focused = buffer.cell((8, 3)).unwrap();
        assert_eq!(focused.symbol(), "s");
        assert_eq!(focused.bg, Color::DarkGray);
        assert!(focused.modifier.contains(Modifier::BOLD));

        let unfocused = buffer.cell((8, 1)).unwrap();
        assert_ne!(unfocused.bg, Color::DarkGray);
    }

    #[test]
    fn render_clamps_an_out_of_bounds_index() {
        let invocations = sample_tree();
        let terminal = render_to_buffer(&invocations, 99, 60, 10);
        let text = buffer_text(&terminal);

        assert!(text.contains("Timeline (4/4)"));
        assert!(text.contains("deposit"));
    }

    #[test]
    fn render_reports_an_empty_timeline() {
        let terminal = render_to_buffer(&[], 0, 60, 10);
        let text = buffer_text(&terminal);

        assert!(text.contains("No contract invocations recorded"));
        assert!(text.contains("Timeline (0/0)"));
    }

    #[test]
    fn short_contract_ids_are_kept_as_they_are() {
        assert_eq!(shorten_contract_id("CB...123"), "CB...123");
        assert_eq!(shorten_contract_id("unknown"), "unknown");
    }

    #[test]
    fn long_contract_ids_are_elided_in_the_middle() {
        let contract_id = "CBQYN5XZ6DK2GSHYPG5M3AEH2M4HKZLK5M5VPGLMIYUV4WXV6RSOMCC5";
        assert_eq!(shorten_contract_id(contract_id), "CBQYN5XZ…MCC5");
    }
}
