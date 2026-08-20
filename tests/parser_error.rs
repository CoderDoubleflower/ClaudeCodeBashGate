mod common;

use bashgate::TooComplexReason;
use common::too_complex;

#[test]
fn malformed_bash_never_masquerades_as_parsed() {
    for source in ["echo 'unterminated", "echo \"unterminated", "a &&", "| a"] {
        let error = too_complex(source);
        assert!(matches!(
            error.reason,
            TooComplexReason::ErrorNode
                | TooComplexReason::UnsupportedNode
                | TooComplexReason::InvalidStructure
        ));
    }
}

#[test]
fn known_dynamic_ast_nodes_are_fail_closed() {
    let cases = [
        ("echo $(date)", "command_substitution"),
        ("cat <(producer)", "process_substitution"),
        ("echo ${NAME}", "expansion"),
        ("echo $NAME", "simple_expansion"),
        ("echo $((1 + 2))", "arithmetic_expansion"),
    ];

    for (source, node_type) in cases {
        let error = too_complex(source);
        assert_eq!(error.reason, TooComplexReason::DynamicExpansion);
        assert_eq!(error.node_type.as_deref(), Some(node_type));
    }
}
