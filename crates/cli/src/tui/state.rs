use grat_core::types::trace::{ContractInvocation, ExecutionTrace, ResourceProfile};

pub struct TuiState {
    pub tx_hash: String,
    pub network: String,
    pub ledger_sequence: u32,
    pub selected_panel: Panel,
    pub scroll_offset: usize,
    pub entries: Vec<TraceEntry>,
    pub invocations: Vec<ContractInvocation>,
    pub resources: ResourceProfile,
}

pub struct TraceEntry {
    pub depth: usize,
    pub contract: String,
    pub function: String,
    pub arguments: Vec<String>,
    pub return_value: Option<String>,
    pub is_error: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Timeline,
    Inspector,
    Controls,
}

impl TuiState {
    pub fn new(trace: ExecutionTrace) -> Self {
        let mut entries = Vec::new();
        append_invocations(&trace.invocations, 0, &mut entries);
        Self {
            tx_hash: trace.tx_hash,
            network: trace.network,
            ledger_sequence: trace.ledger_sequence,
            selected_panel: Panel::Timeline,
            scroll_offset: 0,
            entries,
            invocations: trace.invocations,
            resources: trace.resource_profile,
        }
    }

    pub fn switch_panel(&mut self, reverse: bool) {
        self.selected_panel = match (self.selected_panel, reverse) {
            (Panel::Timeline, false) | (Panel::Controls, true) => Panel::Inspector,
            (Panel::Inspector, false) | (Panel::Timeline, true) => Panel::Controls,
            (Panel::Controls, false) | (Panel::Inspector, true) => Panel::Timeline,
        };
    }

    pub fn next(&mut self) {
        self.scroll_offset = self
            .scroll_offset
            .saturating_add(1)
            .min(self.entries.len().saturating_sub(1));
    }

    pub fn previous(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Advance past descendants of the selected recorded invocation.
    pub fn step_over(&mut self) {
        let Some(current) = self.entries.get(self.scroll_offset) else {
            return;
        };
        self.scroll_offset = self
            .entries
            .iter()
            .enumerate()
            .skip(self.scroll_offset + 1)
            .find(|(_, entry)| entry.depth <= current.depth)
            .map_or(self.entries.len().saturating_sub(1), |(index, _)| index);
    }

    pub fn can_step(&self) -> bool {
        self.scroll_offset.saturating_add(1) < self.entries.len()
    }

    pub fn replay_status(&self) -> String {
        if self.entries.is_empty() {
            "Replay: no invocations".to_owned()
        } else {
            format!(
                "Replay: {} {}/{}",
                if self.can_step() { "Paused" } else { "End" },
                self.scroll_offset + 1,
                self.entries.len()
            )
        }
    }
}

fn append_invocations(
    invocations: &[ContractInvocation],
    depth: usize,
    entries: &mut Vec<TraceEntry>,
) {
    for invocation in invocations {
        entries.push(TraceEntry {
            depth,
            contract: invocation.contract_id.clone(),
            function: invocation.function_name.clone(),
            arguments: invocation.arguments.clone(),
            return_value: invocation.return_value.clone(),
            is_error: invocation.is_error,
        });
        append_invocations(&invocation.sub_invocations, depth + 1, entries);
    }
}

#[cfg(test)]
pub fn fixture() -> TuiState {
    use grat_core::types::trace::{StateDiff, TraceId};
    fn invocation(name: &str, children: Vec<ContractInvocation>) -> ContractInvocation {
        ContractInvocation {
            trace_id: TraceId("fixture".to_owned()),
            contract_id: "CEXAMPLE".to_owned(),
            function_name: name.to_owned(),
            arguments: vec![],
            return_value: None,
            host_calls: vec![],
            sub_invocations: children,
            total_cpu_instructions: 0,
            total_memory_bytes: 0,
            is_error: false,
        }
    }
    TuiState::new(ExecutionTrace {
        tx_hash: "abc123".to_owned(),
        network: "testnet".to_owned(),
        ledger_sequence: 1,
        invocations: vec![
            invocation("parent", vec![invocation("child", vec![])]),
            invocation("sibling", vec![]),
        ],
        state_diff: StateDiff::default(),
        diagnostic_events: vec![],
        resource_profile: ResourceProfile {
            total_cpu: 250,
            cpu_limit: 1000,
            total_memory: 500,
            memory_limit: 1000,
            ..ResourceProfile::default()
        },
    })
}
