use signal_space::{
    AuthorityLevel, IoDirection, IoTransport, PortDirection, PortDtype, SPEC_VERSION,
    SUPPORTED_VERSIONS, StateChart, parse_document, round_trip, validate_document,
};

const AGENT_DOC_FIXTURE: &str =
    include_str!("../../signal-space-spec/fixtures/agent_doc_supervisor.json");
const PATCHBOARD_FIXTURE: &str =
    include_str!("../../signal-space-spec/fixtures/patchboard_attention_router.json");
const IO_RACK_FIXTURE: &str =
    include_str!("../../signal-space-spec/fixtures/patchboard_io_rack.json");

#[test]
fn schema_version_is_0_3_0() {
    assert_eq!(SPEC_VERSION, "0.3.0");
}

#[test]
fn supported_versions_keep_0_2_0_documents_valid() {
    assert!(SUPPORTED_VERSIONS.contains(&"0.2.0"));
    assert!(SUPPORTED_VERSIONS.contains(&"0.3.0"));
}

#[test]
fn validates_agent_doc_fixture() {
    let document = parse_document(AGENT_DOC_FIXTURE).expect("fixture parses");

    validate_document(&document).expect("fixture validates");
    // The agent-doc fixture stays at 0.2.0 — the backward-compatibility proof.
    assert_eq!(document.schema_version, "0.2.0");
    assert_eq!(
        round_trip(&document).unwrap().graph.id,
        "agent_doc.supervisor"
    );
}

#[test]
fn agent_doc_fixture_has_no_ports_or_bindings() {
    let document = parse_document(AGENT_DOC_FIXTURE).expect("fixture parses");
    validate_document(&document).expect("fixture validates");
    assert!(
        document
            .graph
            .nodes
            .iter()
            .all(|n| n.ports.is_empty() && n.io_binding.is_none())
    );
}

#[test]
fn validates_patchboard_fixture() {
    let document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");

    validate_document(&document).expect("fixture validates");
    assert_eq!(document.schema_version, SPEC_VERSION);
    assert!(document.graph.nodes.iter().any(|node| {
        node.allowed_modules
            .iter()
            .any(|module| module == "trainable_model.lifecycle")
    }));
}

#[test]
fn patchboard_fixture_carries_state_charts() {
    let document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
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
fn patchboard_fixture_carries_ports_bindings_and_telemetry() {
    let document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    validate_document(&document).expect("fixture validates");

    let source = find_node(&document, "source.editor");
    // Source exposes a typed output jack and an ingress webhook binding.
    let events_port = source
        .ports
        .iter()
        .find(|p| p.id == "events")
        .expect("events port");
    assert_eq!(events_port.direction, PortDirection::Out);
    assert_eq!(events_port.dtype, PortDtype::Event);
    let ingress = source.io_binding.as_ref().expect("ingress binding");
    assert_eq!(ingress.direction, IoDirection::Ingress);
    assert_eq!(ingress.transport, IoTransport::Webhook);
    assert_eq!(ingress.auth_ref.as_deref(), Some("editor_hook"));

    // Gate carries the egress webhook POST binding (never direct authority).
    let gate = find_node(&document, "gate.approval");
    let egress = gate.io_binding.as_ref().expect("egress binding");
    assert_eq!(egress.direction, IoDirection::Egress);
    assert_ne!(gate.authority.default, AuthorityLevel::Direct);

    // Every edge is a typed cable with live telemetry.
    for edge in &document.graph.edges {
        assert!(edge.from_port.is_some(), "edge {} has from_port", edge.id);
        assert!(edge.to_port.is_some(), "edge {} has to_port", edge.id);
        assert!(
            edge.stream_telemetry.is_some(),
            "edge {} has telemetry",
            edge.id
        );
    }
}

#[test]
fn validates_io_rack_fixture() {
    let document = parse_document(IO_RACK_FIXTURE).expect("fixture parses");
    validate_document(&document).expect("fixture validates");
    assert_eq!(document.schema_version, SPEC_VERSION);
}

#[test]
fn io_rack_fixture_exercises_every_transport() {
    let document = parse_document(IO_RACK_FIXTURE).expect("fixture parses");
    validate_document(&document).expect("fixture validates");

    let transports: Vec<IoTransport> = document
        .graph
        .nodes
        .iter()
        .filter_map(|n| n.io_binding.as_ref().map(|b| b.transport))
        .collect();
    // All eight transports appear at least once.
    for expected in [
        IoTransport::Webhook,
        IoTransport::Websocket,
        IoTransport::FileTail,
        IoTransport::Timer,
        IoTransport::StdinJsonl,
        IoTransport::Exec,
        IoTransport::Notify,
        IoTransport::Mcp,
    ] {
        assert!(
            transports.contains(&expected),
            "io_rack missing transport {:?}",
            expected
        );
    }
}

#[test]
fn rejects_unsupported_spec_version() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    document.schema_version = "1.0.0".to_string();

    let error = validate_document(&document).expect_err("version rejected");
    assert!(error.to_string().contains("unsupported spec version"));
}

#[test]
fn rejects_unknown_edge_endpoint() {
    let mut document = parse_document(AGENT_DOC_FIXTURE).expect("fixture parses");
    document.graph.edges[0].from_node = "missing".to_string();

    let error = validate_document(&document).expect_err("invalid edge rejected");
    assert!(error.to_string().contains("unknown endpoint"));
}

#[test]
fn rejects_silent_direct_authority_escalation() {
    let mut document = parse_document(AGENT_DOC_FIXTURE).expect("fixture parses");
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
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    let model = find_node_mut(&mut document, "model.route_priority");
    let chart = model.state_chart.as_mut().expect("model chart");
    chart.initial = "nope".to_string();

    let error = validate_document(&document).expect_err("bad chart rejected");
    assert!(error.to_string().contains("initial state not in states"));
}

#[test]
fn rejects_state_chart_transition_to_unknown_state() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    let model = find_node_mut(&mut document, "model.route_priority");
    let chart = model.state_chart.as_mut().expect("model chart");
    chart.transitions[0].to = "imaginary".to_string();

    let error = validate_document(&document).expect_err("bad transition rejected");
    assert!(error.to_string().contains("transition 'to' not in states"));
}

#[test]
fn rejects_ingress_binding_on_non_source_node() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    // Move the source's ingress binding onto the memory (window) node.
    let ingress = find_node_mut(&mut document, "source.editor")
        .io_binding
        .take()
        .expect("ingress binding");
    find_node_mut(&mut document, "window.route_features").io_binding = Some(ingress);

    let error = validate_document(&document).expect_err("ingress rejected");
    assert!(
        error
            .to_string()
            .contains("ingress io_binding only allowed on source")
    );
}

#[test]
fn rejects_egress_binding_with_direct_authority() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    let gate = find_node_mut(&mut document, "gate.approval");
    assert!(gate.io_binding.is_some());
    gate.authority.default = AuthorityLevel::Direct;

    let error = validate_document(&document).expect_err("egress direct rejected");
    assert!(
        error
            .to_string()
            .contains("egress io_binding cannot carry direct authority")
    );
}

#[test]
fn rejects_egress_binding_on_non_gate_output_node() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    // Move the gate's egress binding onto a decision node.
    let egress = find_node_mut(&mut document, "gate.approval")
        .io_binding
        .take()
        .expect("egress binding");
    find_node_mut(&mut document, "model.route_priority").io_binding = Some(egress);

    let error = validate_document(&document).expect_err("egress rejected");
    assert!(
        error
            .to_string()
            .contains("egress io_binding only allowed on gate/output")
    );
}

#[test]
fn rejects_edge_port_dtype_mismatch() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    // Rewire the source->window cable into the window's `features` out-jack,
    // which is the wrong direction and dtype — but force a pure dtype mismatch
    // by retyping the window's `events` in-port to scalar.
    let window = find_node_mut(&mut document, "window.route_features");
    let events_in = window
        .ports
        .iter_mut()
        .find(|p| p.id == "events")
        .expect("events in-port");
    events_in.dtype = PortDtype::Scalar;

    let error = validate_document(&document).expect_err("dtype mismatch rejected");
    assert!(error.to_string().contains("port dtype mismatch"));
}

#[test]
fn rejects_edge_from_port_not_found() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    document.graph.edges[0].from_port = Some("imaginary".to_string());

    let error = validate_document(&document).expect_err("missing port rejected");
    assert!(error.to_string().contains("from_port not found"));
}

#[test]
fn rejects_edge_from_port_without_to_port() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    document.graph.edges[0].to_port = None;

    let error = validate_document(&document).expect_err("half-named cable rejected");
    assert!(
        error
            .to_string()
            .contains("names from_port but not to_port")
    );
}

#[test]
fn rejects_duplicate_port_id_on_node() {
    let mut document = parse_document(PATCHBOARD_FIXTURE).expect("fixture parses");
    let source = find_node_mut(&mut document, "source.editor");
    let dup = source.ports[0].clone();
    source.ports.push(dup);

    let error = validate_document(&document).expect_err("duplicate port rejected");
    assert!(error.to_string().contains("duplicate port id"));
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

#[test]
fn telemetry_and_binding_round_trip_through_serde() {
    use signal_space::{IoBinding, StreamTelemetry};
    let telemetry = StreamTelemetry {
        rate_hz: Some(2.0),
        latency_ms: None,
        freshness_ms: Some(50.0),
        last_value_preview: Some(serde_json::json!({ "v": 1 })),
        distribution_hint: None,
        missing_data: false,
    };
    let json = serde_json::to_string(&telemetry).unwrap();
    // None fields are omitted, keeping cables compact on the wire.
    assert!(!json.contains("latency_ms"));
    let back: StreamTelemetry = serde_json::from_str(&json).unwrap();
    assert_eq!(telemetry, back);

    let binding = IoBinding {
        direction: IoDirection::Egress,
        transport: IoTransport::Notify,
        endpoint: None,
        format: Some("text".to_string()),
        schema_ref: None,
        auth_ref: None,
    };
    let json = serde_json::to_string(&binding).unwrap();
    let back: IoBinding = serde_json::from_str(&json).unwrap();
    assert_eq!(binding, back);
}

fn find_node<'a>(
    document: &'a signal_space::SignalSpaceDocument,
    id: &str,
) -> &'a signal_space::SignalNode {
    document
        .graph
        .nodes
        .iter()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("node {id}"))
}

fn find_node_mut<'a>(
    document: &'a mut signal_space::SignalSpaceDocument,
    id: &str,
) -> &'a mut signal_space::SignalNode {
    document
        .graph
        .nodes
        .iter_mut()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("node {id}"))
}
