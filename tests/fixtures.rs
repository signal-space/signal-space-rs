use signal_space::{SPEC_VERSION, parse_document, round_trip, validate_document};

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
fn rejects_unknown_edge_endpoint() {
    let mut document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/agent_doc_supervisor.json"
    ))
    .expect("fixture parses");
    document.graph.edges[0].from_node = "missing".to_string();

    let error = validate_document(&document).expect_err("invalid edge rejected");
    assert!(error.to_string().contains("unknown endpoint"));
}
