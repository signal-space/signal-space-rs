#![cfg(feature = "lazily-runtime")]

use lazily::{DeltaApplyStatus, NodeState};
use signal_space::runtime::{SignalSpaceRuntime, timeline_by_class};
use signal_space::{StateClass, parse_document, validate_document};

fn patchboard_graph() -> signal_space::SignalGraph {
    let document = parse_document(include_str!(
        "../../signal-space-spec/fixtures/patchboard_attention_router.json"
    ))
    .expect("fixture parses");
    validate_document(&document).expect("fixture validates");
    document.graph
}

#[test]
fn lazily_runtime_updates_derived_inspectors_from_graph_cell_changes() {
    let runtime = SignalSpaceRuntime::new(patchboard_graph());

    let before = runtime
        .inspectors()
        .into_iter()
        .find(|inspector| inspector.node_id == "model.route_priority")
        .expect("model inspector exists");
    assert_eq!(
        before.state_summary,
        "Scores whether a human or agent should review next"
    );
    assert_eq!(
        before.writable_fields,
        vec!["model.active_candidate".to_string()]
    );
    assert_eq!(before.derived_fields, vec!["model.confidence".to_string()]);

    runtime.update_graph(|graph| {
        let node = graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "model.route_priority")
            .expect("model node exists");
        node.state.summary = "Promoted candidate awaiting gated rollout".to_string();
        node.allowed_intents.push("archive_candidate".to_string());
    });

    let after = runtime
        .inspectors()
        .into_iter()
        .find(|inspector| inspector.node_id == "model.route_priority")
        .expect("model inspector exists");
    assert_eq!(
        after.state_summary,
        "Promoted candidate awaiting gated rollout"
    );
    assert!(
        after
            .allowed_intents
            .iter()
            .any(|intent| intent == "archive_candidate")
    );
}

#[test]
fn lazily_runtime_exports_read_only_snapshot_and_delta_projection() {
    let graph = patchboard_graph();
    let runtime = SignalSpaceRuntime::new(graph.clone());
    let mirror = runtime.mirror_export();

    assert_eq!(
        mirror.snapshot.nodes.len(),
        1 + graph.nodes.len() + graph.timeline.len()
    );
    assert_eq!(mirror.snapshot.roots.len(), 1);
    assert_eq!(mirror.delta.base_epoch, 0);
    assert_eq!(mirror.delta.epoch, 1);
    assert_eq!(mirror.delta.apply_status(0), DeltaApplyStatus::Apply);
    assert_eq!(
        mirror.delta.ops.len(),
        1 + graph.nodes.len() + graph.timeline.len()
    );
    assert!(
        mirror
            .snapshot
            .nodes
            .iter()
            .all(|node| { matches!(node.state, NodeState::Payload(_)) })
    );
}

#[test]
fn timeline_projection_keeps_observations_recommendations_and_actions_distinct() {
    let graph = patchboard_graph();

    assert_eq!(timeline_by_class(&graph, StateClass::Observation).len(), 1);
    assert_eq!(
        timeline_by_class(&graph, StateClass::Recommendation).len(),
        2
    );
    assert_eq!(timeline_by_class(&graph, StateClass::Action).len(), 0);
}
