#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    pub max_command_bytes: usize,
    pub max_nodes: usize,
    pub max_commands: usize,
    pub max_depth: usize,
    pub parse_timeout_micros: u64,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_command_bytes: 10_000,
            max_nodes: 50_000,
            max_commands: 1_024,
            max_depth: 256,
            parse_timeout_micros: 50_000,
        }
    }
}

impl ParseLimits {
    pub(crate) const HARD_MAX_DEPTH: usize = 512;
    pub(crate) const HARD_MAX_NODES: usize = 1_000_000;

    #[must_use]
    pub(crate) fn effective_max_depth(self) -> usize {
        self.max_depth.min(Self::HARD_MAX_DEPTH)
    }

    #[must_use]
    pub(crate) fn effective_max_nodes(self) -> usize {
        self.max_nodes.min(Self::HARD_MAX_NODES)
    }

    #[must_use]
    pub(crate) fn effective_timeout_micros(self) -> u64 {
        self.parse_timeout_micros.max(1)
    }
}
