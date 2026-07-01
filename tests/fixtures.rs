use signal_space::{
    AuthorityLevel, SPEC_VERSION, StateChart, parse_document, round_trip, validate_document,
};

#[test]
fn schema_version_is_0_2_0() {
    assert_eq!(SPEC_VERSION, "0.2.0");
}

#[test]
fn validates_agent_doc_fixture() {
    let document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/agent_doc_supervisor.json"
    ))
    .expect("fixture parses");

    validate_document(&document).expect("fixture validates");
    assert_eq!(document.schema_version, SPEC_VERSION);
    assert_eq!(
        round_trip(&document).unwrap().graph.id,
        "agent_doc.supervisor"
    );
}

#[test]
fn validates_patchboard_fixture() {
    let document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/patchboard_attention_router.json"
    ))
    .expect("fixture parses");

    validate_document(&document).expect("fixture validates");
    assert!(document.graph.nodes.iter().any(|node| {
        node.allowed_modules
            .iter()
            .any(|module| module == "trainable_model.lifecycle")
    }));
}

#[test]
fn patchboard_fixture_carries_state_charts() {
    let document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/patchboard_attention_router.json"
    ))
    .expect("fixture parses");
    validate_document(&document).expect("fixture validates");

    // The model, gate, and agent nodes declare a StateChart; source/window do not.
    let charts: Vec<_> = document
        .graph
        .nodes
        .iter()
        .filter_map(|n| n.state_chart.as_ref().map(|c| (n.id.as_str(), c)))
        .collect();
    let ids: Vec<&str> = charts.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        ids,
        [
            "model.route_priority",
            "gate.approval",
            "agent.review_patch"
        ]
    );

    let (_, model_chart) = charts
        .iter()
        .find(|(id, _)| *id == "model.route_priority")
        .expect("model chart");
    assert_eq!(model_chart.effective_current(), "shadow");
    assert_eq!(
        model_chart.transition("shadow", "promote_candidate"),
        Some("active")
    );
    assert_eq!(
        model_chart.transition("active", "rollback_model"),
        Some("rolled_back")
    );
    assert_eq!(
        model_chart.transition("shadow", "rollback_model"),
        Some("rolled_back")
    );
    // An undefined event returns None (mirrors lazily's StateMachine::on).
    assert_eq!(
        model_chart.transition("shadow", "start_candidate_shadow"),
        None
    );

    // Round-trip preserves the charts byte-for-byte structurally.
    let round_tripped = round_trip(&document).unwrap();
    for (original, rebound) in document
        .graph
        .nodes
        .iter()
        .zip(round_tripped.graph.nodes.iter())
    {
        assert_eq!(original.state_chart, rebound.state_chart);
    }
}

#[test]
fn rejects_unknown_edge_endpoint() {
    let mut document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/agent_doc_supervisor.json"
    ))
    .expect("fixture parses");
    document.graph.edges[0].from_node = "missing".to_string();

    let error = validate_document(&document).expect_err("invalid edge rejected");
    assert!(error.to_string().contains("unknown endpoint"));
}

#[test]
fn rejects_silent_direct_authority_escalation() {
    let mut document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/agent_doc_supervisor.json"
    ))
    .expect("fixture parses");
    let intent = &mut document.graph.nodes[2]
        .decision
        .as_mut()
        .expect("decision node")
        .proposed_intents[0];
    intent.authority = AuthorityLevel::Direct;

    let error = validate_document(&document).expect_err("invalid authority rejected");
    assert!(error.to_string().contains("direct authority"));
}

#[test]
fn rejects_state_chart_with_unknown_initial() {
    let mut document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/patchboard_attention_router.json"
    ))
    .expect("fixture parses");
    let model = document
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == "model.route_priority")
        .expect("model node");
    let chart = model.state_chart.as_mut().expect("model chart");
    chart.initial = "nope".to_string();

    let error = validate_document(&document).expect_err("bad chart rejected");
    assert!(error.to_string().contains("initial state not in states"));
}

#[test]
fn rejects_state_chart_transition_to_unknown_state() {
    let mut document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/patchboard_attention_router.json"
    ))
    .expect("fixture parses");
    let model = document
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == "model.route_priority")
        .expect("model node");
    let chart = model.state_chart.as_mut().expect("model chart");
    // Corrupt the first transition's destination.
    chart.transitions[0].to = "imaginary".to_string();

    let error = validate_document(&document).expect_err("bad transition rejected");
    assert!(error.to_string().contains("transition 'to' not in states"));
}

#[test]
fn state_chart_helpers_round_trip_through_serde() {
    let chart = StateChart {
        states: vec!["idle".to_string(), "running".to_string()],
        initial: "idle".to_string(),
        current: None,
        transitions: vec![signal_space::StateTransition {
            from: "idle".to_string(),
            event: "start".to_string(),
            to: "running".to_string(),
        }],
    };
    let json = serde_json::to_string(&chart).unwrap();
    // `current` is absent when None (skip_serializing_if).
    assert!(!json.contains("current"));
    let back: StateChart = serde_json::from_str(&json).unwrap();
    assert_eq!(chart, back);
    assert_eq!(chart.effective_current(), "idle");
}
