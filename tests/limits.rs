use bashgate::{BashGate, GateResult, ParseLimits, TooComplexReason};

fn limit_error(gate: &BashGate, source: &str) -> bashgate::TooComplex {
    match gate.parse(source) {
        GateResult::TooComplex(error) => error,
        other => panic!("expected limit failure for {source:?}, got {other:?}"),
    }
}

#[test]
fn enforces_max_command_bytes() {
    let gate = BashGate::new(ParseLimits {
        max_command_bytes: 4,
        ..ParseLimits::default()
    });
    let error = limit_error(&gate, "echo x");
    assert_eq!(error.reason, TooComplexReason::LimitExceeded);
    assert_eq!(error.node_type.as_deref(), Some("max_command_bytes"));
}

#[test]
fn enforces_max_commands() {
    let gate = BashGate::new(ParseLimits {
        max_commands: 1,
        ..ParseLimits::default()
    });
    let error = limit_error(&gate, "a ; b");
    assert_eq!(error.reason, TooComplexReason::LimitExceeded);
    assert_eq!(error.node_type.as_deref(), Some("max_commands"));
}

#[test]
fn enforces_ast_node_budget() {
    let gate = BashGate::new(ParseLimits {
        max_nodes: 1,
        ..ParseLimits::default()
    });
    let error = limit_error(&gate, "echo hello");
    assert_eq!(error.reason, TooComplexReason::LimitExceeded);
    assert_eq!(error.node_type.as_deref(), Some("max_nodes"));
}

#[test]
fn enforces_ast_depth_budget() {
    let gate = BashGate::new(ParseLimits {
        max_depth: 0,
        ..ParseLimits::default()
    });
    let error = limit_error(&gate, "echo hello");
    assert_eq!(error.reason, TooComplexReason::LimitExceeded);
    assert_eq!(error.node_type.as_deref(), Some("max_depth"));
}
