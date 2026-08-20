mod common;

use common::parsed;
use serde_json::json;

#[test]
fn gate_result_json_matches_the_public_contract() {
    let result = bashgate::parse("git status && echo ok");
    let value = serde_json::to_value(result).expect("serialize GateResult");

    assert_eq!(value["kind"], "parsed");
    assert_eq!(value["operators"], json!(["&&"]));
    assert_eq!(value["commands"][0]["argv"], json!(["git", "status"]));
    assert_eq!(value["commands"][1]["argv"], json!(["echo", "ok"]));
}

#[test]
fn too_complex_json_is_flat_and_machine_readable() {
    let value = serde_json::to_value(bashgate::parse("for x in a; do x; done"))
        .expect("serialize TooComplex result");

    assert_eq!(value["kind"], "too_complex");
    assert_eq!(value["reason"], "unsupported_node");
    assert_eq!(value["node_type"], "for_statement");
}

#[test]
fn operator_spans_remain_available_in_rust_ir() {
    let program = parsed("a && b");
    assert_eq!(program.operators[0].span.start_byte, 2);
    assert_eq!(program.operators[0].span.end_byte, 4);
}
