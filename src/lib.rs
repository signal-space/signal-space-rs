use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// The latest Signal Space schema version this binding implements.
pub const SPEC_VERSION: &str = "0.4.0";

/// Every schema version [`validate_document`] accepts. `0.2.0` documents keep
/// validating because every later addition (`state_chart`, `ports`,
/// `stream_telemetry`, `io_binding`, edge `from_port`/`to_port`, and the
/// `0.4.0` graph-level `category`/`tags`) is optional.
pub const SUPPORTED_VERSIONS: &[&str] = &["0.2.0", "0.3.0", "0.4.0"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityLevel {
    Local,
    Advisory,
    Gated,
    Direct,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateClass {
    Observation,
    Recommendation,
    Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeFamily {
    Source,
    Transform,
    Memory,
    Decision,
    Gate,
    Output,
}

/// Direction of a typed jack on a module. Added in `0.3.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    In,
    Out,
}

/// Jack dtype. Cables should connect compatible dtypes; a host rejects
/// mismatched patches visibly. Added in `0.3.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDtype {
    Scalar,
    Vector,
    Event,
    Window,
    Decision,
    Label,
}

/// A typed jack on a module. Edges reference ports via `from_port`/`to_port`
/// so the patchboard can dtype-check cables and render live telemetry per jack.
/// Added in `0.3.0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortSpec {
    pub id: String,
    pub name: Option<String>,
    pub direction: PortDirection,
    pub dtype: PortDtype,
    #[serde(default)]
    pub required: bool,
}

/// Live-cable readout for an edge. Always derived/observed state, never a
/// writable cell; connector failures surface here as `missing_data` / stale
/// freshness. Added in `0.3.0`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamTelemetry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_hz: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_value_preview: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distribution_hint: Option<serde_json::Value>,
    #[serde(default)]
    pub missing_data: bool,
}

/// Direction of an external transport binding. Added in `0.3.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoDirection {
    Ingress,
    Egress,
}

/// How a source/output node binds to the outside world. Secrets are never
/// inlined; `auth_ref` names a host-resolved credential. Added in `0.3.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoTransport {
    Webhook,
    Websocket,
    FileTail,
    Timer,
    StdinJsonl,
    Exec,
    Notify,
    Mcp,
}

/// Declares how a node binds to an external transport. Ingress bindings are
/// restricted to `source` nodes; egress bindings to `gate`/`output` nodes;
/// egress must never carry `direct` authority. Added in `0.3.0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoBinding {
    pub direction: IoDirection,
    pub transport: IoTransport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalSpaceDocument {
    pub schema_version: String,
    pub graph: SignalGraph,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalGraph {
    pub id: String,
    pub name: Option<String>,
    pub source: Option<String>,
    pub nodes: Vec<SignalNode>,
    pub edges: Vec<SignalEdge>,
    pub timeline: Vec<TimelineEvent>,
    #[serde(default)]
    pub inspectors: Vec<serde_json::Value>,
    pub intent_modules: Vec<IntentModule>,
    #[serde(default)]
    pub allowed_intents: Vec<String>,
    pub authority: Authority,
    pub snapshot: Option<serde_json::Value>,
    /// Optional catalog category for the graph (conventionally
    /// `"workflow_template"` for an n8n-style template). Added in `0.4.0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Optional free-form tags describing the graph's triggers/actions. `0.4.0`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalNode {
    pub id: String,
    pub family: NodeFamily,
    pub mode: Option<String>,
    pub label: Option<String>,
    pub authority: Authority,
    pub state: NodeState,
    #[serde(default)]
    pub recent_events: Vec<SignalEvent>,
    pub explanation: Option<serde_json::Value>,
    pub decision: Option<DecisionEnvelope>,
    #[serde(default)]
    pub allowed_modules: Vec<String>,
    pub allowed_intents: Vec<String>,
    /// Optional declarative state machine (added in `0.2.0`). When present, an
    /// adapter SHOULD route transitions through [`StateChart::transition`] and
    /// keep `current` in sync with its live runtime state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_chart: Option<StateChart>,
    /// Typed jacks (added in `0.3.0`). Empty for nodes that predate ports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<PortSpec>,
    /// External transport binding (added in `0.3.0`). Ingress on sources,
    /// egress on gates/outputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub io_binding: Option<IoBinding>,
    pub provenance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeState {
    pub summary: String,
    #[serde(default)]
    pub fields: Vec<StateField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateField {
    pub id: String,
    pub state_class: StateClass,
    pub writable: bool,
    #[serde(default)]
    pub derived: bool,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalEdge {
    pub id: String,
    #[serde(rename = "from")]
    pub from_node: String,
    #[serde(rename = "to")]
    pub to_node: String,
    pub label: Option<String>,
    pub authority: Option<Authority>,
    #[serde(default)]
    pub state_fields: Vec<StateField>,
    /// Output jack on the `from` node (added in `0.3.0`). Absent on edges that
    /// predate typed ports.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_port: Option<String>,
    /// Input jack on the `to` node (added in `0.3.0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_port: Option<String>,
    /// Live-cable readout (added in `0.3.0`). Derived only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_telemetry: Option<StreamTelemetry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalEvent {
    pub id: String,
    pub kind: String,
    pub state_class: StateClass,
    pub created_at: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub evidence: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: String,
    pub kind: String,
    pub state_class: StateClass,
    pub created_at: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub evidence: Vec<serde_json::Value>,
    pub node_id: String,
    pub edge_id: Option<String>,
    pub run_id: Option<String>,
    pub decision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionEnvelope {
    pub id: String,
    pub mode: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub output_schema: serde_json::Value,
    pub confidence: Option<f64>,
    #[serde(default)]
    pub explanation: serde_json::Value,
    #[serde(default)]
    pub proposed_intents: Vec<SurfaceIntent>,
    pub authority: AuthorityLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceIntent {
    pub id: String,
    #[serde(rename = "type")]
    pub intent_type: String,
    pub target: Target,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub actor: String,
    pub authority: AuthorityLevel,
    pub reason: Option<String>,
    #[serde(default)]
    pub evidence: Vec<serde_json::Value>,
    pub created_at: String,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Authority {
    pub default: AuthorityLevel,
    #[serde(default)]
    pub by_intent: std::collections::BTreeMap<String, AuthorityLevel>,
    pub owner: Option<String>,
    pub boundary: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentModule {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    pub capability: Option<String>,
    #[serde(default)]
    pub intents: Vec<IntentSchema>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentSchema {
    #[serde(rename = "type")]
    pub intent_type: String,
    pub authority: AuthorityLevel,
    #[serde(default)]
    pub target_families: Vec<String>,
    #[serde(default)]
    pub payload_schema: serde_json::Value,
    pub description: Option<String>,
}

/// A declarative finite state machine mirroring lazily's reactive
/// `StateMachine<S, E>`.
///
/// Added in schema `0.2.0` as an optional field on [`SignalNode`]. The chart
/// declares the enumeration of valid states, the entry state, an optional live
/// state, and the transition function expressed as `{from, event, to}` triples.
/// The owning adapter remains responsible for applying or rejecting any
/// transition at its authority boundary; a binding that targets only `0.1.0`
/// MAY ignore the field entirely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateChart {
    pub states: Vec<String>,
    pub initial: String,
    /// Live state of the machine, mirroring lazily's `state()` getter. Absent
    /// when the live state is carried by `state.fields` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<String>,
    pub transitions: Vec<StateTransition>,
}

/// A single [`StateChart`] edge: the machine moves from `from` to `to` when it
/// observes `event`. The `event` conventionally matches an intent type
/// advertised by the graph so a `SurfaceIntent` doubles as a state-machine
/// event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub event: String,
    pub to: String,
}

impl StateChart {
    /// Returns the destination state for `event` fired from `from`, or `None`
    /// when no transition matches. Mirrors lazily's `StateMachine::on` returning
    /// `Option<S>`; a `None` result is not an error, it merely means the event
    /// is undefined for the current state.
    pub fn transition(&self, from: &str, event: &str) -> Option<&str> {
        self.transitions
            .iter()
            .find(|t| t.from == from && t.event == event)
            .map(|t| t.to.as_str())
    }

    /// The effective live state: `current` when set, otherwise `initial`.
    pub fn effective_current(&self) -> &str {
        self.current.as_deref().unwrap_or(&self.initial)
    }

    /// Whether `state` is one of the declared `states`.
    pub fn contains_state(&self, state: &str) -> bool {
        self.states.iter().any(|s| s == state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ValidationError {}

pub fn parse_document(input: &str) -> Result<SignalSpaceDocument, serde_json::Error> {
    serde_json::from_str(input)
}

pub fn validate_document(document: &SignalSpaceDocument) -> Result<(), ValidationError> {
    if !SUPPORTED_VERSIONS.contains(&document.schema_version.as_str()) {
        return Err(ValidationError::new(format!(
            "unsupported spec version: {}",
            document.schema_version
        )));
    }

    let mut nodes_by_id: std::collections::HashMap<&str, &SignalNode> =
        std::collections::HashMap::with_capacity(document.graph.nodes.len());
    for node in &document.graph.nodes {
        if nodes_by_id.insert(node.id.as_str(), node).is_some() {
            return Err(ValidationError::new(format!(
                "duplicate node id: {}",
                node.id
            )));
        }
        for field in &node.state.fields {
            if field.derived && field.writable {
                return Err(ValidationError::new(format!(
                    "derived field cannot be writable: {}",
                    field.id
                )));
            }
        }
        if node
            .allowed_modules
            .iter()
            .any(|module| module == "trainable_model.lifecycle")
        {
            let has_capability = node.decision.as_ref().is_some_and(|decision| {
                decision
                    .capabilities
                    .iter()
                    .any(|capability| capability == "trainable_model.lifecycle")
            });
            if !has_capability {
                return Err(ValidationError::new(format!(
                    "trainable lifecycle advertised without decision capability: {}",
                    node.id
                )));
            }
        }
        if let Some(decision) = &node.decision {
            for intent in &decision.proposed_intents {
                if intent.authority == AuthorityLevel::Direct
                    && !grants_direct_authority(node, decision, intent)
                {
                    return Err(ValidationError::new(format!(
                        "proposed intent upgrades to direct authority: {}",
                        intent.id
                    )));
                }
            }
        }
        if let Some(chart) = &node.state_chart {
            validate_state_chart(node.id.as_str(), chart)?;
        }
        validate_ports(node)?;
        validate_io_binding(node)?;
    }

    let mut edge_ids = HashSet::new();
    for edge in &document.graph.edges {
        if !edge_ids.insert(edge.id.as_str()) {
            return Err(ValidationError::new(format!(
                "duplicate edge id: {}",
                edge.id
            )));
        }
        let from_node = match nodes_by_id.get(edge.from_node.as_str()) {
            Some(node) => *node,
            None => {
                return Err(ValidationError::new(format!(
                    "edge has unknown endpoint: {}",
                    edge.id
                )));
            }
        };
        let to_node = match nodes_by_id.get(edge.to_node.as_str()) {
            Some(node) => *node,
            None => {
                return Err(ValidationError::new(format!(
                    "edge has unknown endpoint: {}",
                    edge.id
                )));
            }
        };
        validate_edge_ports(edge, from_node, to_node)?;
    }

    Ok(())
}

/// Ensure port ids are unique within a node. Added in `0.3.0`.
fn validate_ports(node: &SignalNode) -> Result<(), ValidationError> {
    let mut seen = HashSet::new();
    for port in &node.ports {
        if !seen.insert(port.id.as_str()) {
            return Err(ValidationError::new(format!(
                "duplicate port id: {} on node {}",
                port.id, node.id
            )));
        }
    }
    Ok(())
}

/// Enforce the IoBinding authority boundary. Ingress binds to `source` nodes;
/// egress binds to `gate`/`output` nodes; egress never carries `direct`
/// authority. Added in `0.3.0`.
fn validate_io_binding(node: &SignalNode) -> Result<(), ValidationError> {
    let Some(binding) = &node.io_binding else {
        return Ok(());
    };
    match binding.direction {
        IoDirection::Ingress => {
            if node.family != NodeFamily::Source {
                return Err(ValidationError::new(format!(
                    "ingress io_binding only allowed on source nodes: {}",
                    node.id
                )));
            }
        }
        IoDirection::Egress => {
            if !matches!(node.family, NodeFamily::Gate | NodeFamily::Output) {
                return Err(ValidationError::new(format!(
                    "egress io_binding only allowed on gate/output nodes: {}",
                    node.id
                )));
            }
            if node.authority.default == AuthorityLevel::Direct {
                return Err(ValidationError::new(format!(
                    "egress io_binding cannot carry direct authority: {}",
                    node.id
                )));
            }
        }
    }
    Ok(())
}

/// When an edge names its jacks, both ports must exist; the `from` jack must be
/// an output and the `to` jack an input; and the dtypes must match so the UI
/// can dtype-check a patch visibly. Added in `0.3.0`.
fn validate_edge_ports(
    edge: &SignalEdge,
    from_node: &SignalNode,
    to_node: &SignalNode,
) -> Result<(), ValidationError> {
    let from_port = match &edge.from_port {
        Some(id) => match from_node.ports.iter().find(|p| &p.id == id) {
            Some(port) => port,
            None => {
                return Err(ValidationError::new(format!(
                    "edge from_port not found on {}: {}",
                    from_node.id, edge.id
                )));
            }
        },
        None => return Ok(()),
    };
    let to_port_id = match &edge.to_port {
        Some(id) => id,
        None => {
            return Err(ValidationError::new(format!(
                "edge names from_port but not to_port: {}",
                edge.id
            )));
        }
    };
    let to_port = match to_node.ports.iter().find(|p| &p.id == to_port_id) {
        Some(port) => port,
        None => {
            return Err(ValidationError::new(format!(
                "edge to_port not found on {}: {}",
                to_node.id, edge.id
            )));
        }
    };
    if from_port.direction != PortDirection::Out {
        return Err(ValidationError::new(format!(
            "edge from_port is not an output jack: {}",
            edge.id
        )));
    }
    if to_port.direction != PortDirection::In {
        return Err(ValidationError::new(format!(
            "edge to_port is not an input jack: {}",
            edge.id
        )));
    }
    if from_port.dtype != to_port.dtype {
        return Err(ValidationError::new(format!(
            "edge port dtype mismatch ({} -> {}): {}",
            from_port.dtype_as_str(),
            to_port.dtype_as_str(),
            edge.id
        )));
    }
    Ok(())
}

impl PortSpec {
    fn dtype_as_str(&self) -> &'static str {
        match self.dtype {
            PortDtype::Scalar => "scalar",
            PortDtype::Vector => "vector",
            PortDtype::Event => "event",
            PortDtype::Window => "window",
            PortDtype::Decision => "decision",
            PortDtype::Label => "label",
        }
    }
}

fn grants_direct_authority(
    node: &SignalNode,
    decision: &DecisionEnvelope,
    intent: &SurfaceIntent,
) -> bool {
    node.authority.default == AuthorityLevel::Direct
        || node
            .authority
            .by_intent
            .get(&intent.intent_type)
            .is_some_and(|level| level == &AuthorityLevel::Direct)
        || decision.authority == AuthorityLevel::Direct
}

/// Validate a node's optional `StateChart`: the initial state and every
/// transition endpoint must be declared in `states`, and the live `current`
/// (when present) must likewise be a known state. The transition function is
/// otherwise free-form so an adapter can model idempotent or competing events
/// without contradicting the schema.
fn validate_state_chart(node_id: &str, chart: &StateChart) -> Result<(), ValidationError> {
    if chart.states.is_empty() {
        return Err(ValidationError::new(format!(
            "state_chart has no states: {node_id}"
        )));
    }
    if !chart.contains_state(&chart.initial) {
        return Err(ValidationError::new(format!(
            "state_chart initial state not in states: {node_id}"
        )));
    }
    if let Some(current) = &chart.current
        && !chart.contains_state(current)
    {
        return Err(ValidationError::new(format!(
            "state_chart current state not in states: {node_id}"
        )));
    }
    for transition in &chart.transitions {
        if !chart.contains_state(&transition.from) {
            return Err(ValidationError::new(format!(
                "state_chart transition 'from' not in states: {node_id}"
            )));
        }
        if !chart.contains_state(&transition.to) {
            return Err(ValidationError::new(format!(
                "state_chart transition 'to' not in states: {node_id}"
            )));
        }
    }
    Ok(())
}

pub fn round_trip(
    document: &SignalSpaceDocument,
) -> Result<SignalSpaceDocument, serde_json::Error> {
    serde_json::from_value(serde_json::to_value(document)?)
}

#[cfg(feature = "lazily-runtime")]
pub mod runtime {
    use std::rc::Rc;

    use lazily::{
        CellHandle, Context, Delta, DeltaOp, EdgeSnapshot, IpcValue, NodeId, NodeSnapshot,
        SignalHandle, Snapshot,
    };

    use crate::{
        SignalGraph, SignalNode, StateClass, StateField, TimelineEvent, validate_document,
    };

    /// Summary row for an inspector panel derived from graph state.
    ///
    /// This is intentionally derived-only. Mutating node state must go through
    /// the graph cell and then the owning adapter's authority boundary.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InspectorSummary {
        pub node_id: String,
        pub family: String,
        pub state_summary: String,
        pub authority: String,
        pub allowed_intents: Vec<String>,
        pub writable_fields: Vec<String>,
        pub derived_fields: Vec<String>,
        pub recent_event_count: usize,
    }

    /// Read-only mirror projection over lazily Snapshot/Delta IPC types.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MirrorExport {
        pub snapshot: Snapshot,
        pub delta: Delta,
    }

    /// Lazily-backed local runtime for a Signal Space graph.
    ///
    /// The graph itself is the only writable cell. Inspector summaries and
    /// mirror exports are eager signals derived from that cell.
    pub struct SignalSpaceRuntime {
        ctx: Context,
        graph: CellHandle<SignalGraph>,
        inspectors: SignalHandle<Vec<InspectorSummary>>,
        mirror: SignalHandle<MirrorExport>,
    }

    impl SignalSpaceRuntime {
        pub fn new(graph: SignalGraph) -> Self {
            let ctx = Context::new();
            let graph_cell = ctx.cell(graph);
            let inspectors_cell = graph_cell;
            let inspectors = ctx.signal(move |ctx| {
                let graph: Rc<SignalGraph> = ctx.get_cell_rc(&inspectors_cell);
                inspector_summaries(&graph)
            });
            let mirror_cell = graph_cell;
            let mirror = ctx.signal(move |ctx| {
                let graph: Rc<SignalGraph> = ctx.get_cell_rc(&mirror_cell);
                mirror_export(&graph)
            });
            Self {
                ctx,
                graph: graph_cell,
                inspectors,
                mirror,
            }
        }

        pub fn graph(&self) -> SignalGraph {
            self.ctx.get_cell(&self.graph)
        }

        pub fn set_graph(&self, graph: SignalGraph) {
            self.graph.set(&self.ctx, graph);
        }

        pub fn update_graph(&self, update: impl FnOnce(&mut SignalGraph)) {
            let mut graph = self.graph();
            update(&mut graph);
            self.set_graph(graph);
        }

        pub fn inspectors(&self) -> Vec<InspectorSummary> {
            self.inspectors.get(&self.ctx)
        }

        pub fn mirror_export(&self) -> MirrorExport {
            self.mirror.get(&self.ctx)
        }
    }

    pub fn inspector_summaries(graph: &SignalGraph) -> Vec<InspectorSummary> {
        graph.nodes.iter().map(inspector_summary).collect()
    }

    pub fn inspector_summary(node: &SignalNode) -> InspectorSummary {
        InspectorSummary {
            node_id: node.id.clone(),
            family: family_as_str(node.family).to_string(),
            state_summary: node.state.summary.clone(),
            authority: authority_level_as_str(node.authority.default).to_string(),
            allowed_intents: node.allowed_intents.clone(),
            writable_fields: fields_by_kind(&node.state.fields, true),
            derived_fields: fields_by_kind(&node.state.fields, false),
            recent_event_count: node.recent_events.len(),
        }
    }

    fn family_as_str(family: crate::NodeFamily) -> &'static str {
        match family {
            crate::NodeFamily::Source => "source",
            crate::NodeFamily::Transform => "transform",
            crate::NodeFamily::Memory => "memory",
            crate::NodeFamily::Decision => "decision",
            crate::NodeFamily::Gate => "gate",
            crate::NodeFamily::Output => "output",
        }
    }

    fn authority_level_as_str(level: crate::AuthorityLevel) -> &'static str {
        match level {
            crate::AuthorityLevel::Local => "local",
            crate::AuthorityLevel::Advisory => "advisory",
            crate::AuthorityLevel::Gated => "gated",
            crate::AuthorityLevel::Direct => "direct",
        }
    }

    fn fields_by_kind(fields: &[StateField], writable: bool) -> Vec<String> {
        fields
            .iter()
            .filter(|field| {
                if writable {
                    field.writable && !field.derived
                } else {
                    field.derived
                }
            })
            .map(|field| field.id.clone())
            .collect()
    }

    pub fn mirror_export(graph: &SignalGraph) -> MirrorExport {
        let snapshot = snapshot_for_graph(graph);
        let delta = Delta::next(0, delta_ops_for_graph(graph));
        MirrorExport { snapshot, delta }
    }

    pub fn snapshot_for_graph(graph: &SignalGraph) -> Snapshot {
        let total = 1 + graph.nodes.len() + graph.timeline.len();
        let mut nodes = Vec::with_capacity(total);
        nodes.push(node_payload(1, "signal-space.graph", graph));

        let mut wire_ids: std::collections::HashMap<&str, u64> =
            std::collections::HashMap::with_capacity(graph.nodes.len());
        for (idx, node) in graph.nodes.iter().enumerate() {
            let wire_id = node_wire_id(idx);
            wire_ids.insert(node.id.as_str(), wire_id);
            nodes.push(node_payload(wire_id, "signal-space.node", node));
        }
        for (idx, event) in graph.timeline.iter().enumerate() {
            nodes.push(node_payload(
                event_wire_id(graph.nodes.len(), idx),
                "signal-space.timeline_event",
                event,
            ));
        }

        let edges = graph
            .edges
            .iter()
            .filter_map(|edge| {
                let dependent = wire_ids.get(edge.to_node.as_str()).copied()?;
                let dependency = wire_ids.get(edge.from_node.as_str()).copied()?;
                Some(EdgeSnapshot::new(NodeId(dependent), NodeId(dependency)))
            })
            .collect();

        Snapshot::new(0, nodes, edges, vec![NodeId(1)])
    }

    pub fn delta_ops_for_graph(graph: &SignalGraph) -> Vec<DeltaOp> {
        let total = 1 + graph.nodes.len() + graph.timeline.len();
        let mut ops = Vec::with_capacity(total);
        ops.push(DeltaOp::SlotValue {
            node: NodeId(1),
            payload: to_ipc_value(graph),
        });
        for (idx, node) in graph.nodes.iter().enumerate() {
            ops.push(DeltaOp::SlotValue {
                node: NodeId(node_wire_id(idx)),
                payload: to_ipc_value(node),
            });
        }
        for (idx, event) in graph.timeline.iter().enumerate() {
            ops.push(DeltaOp::SlotValue {
                node: NodeId(event_wire_id(graph.nodes.len(), idx)),
                payload: to_ipc_value(event),
            });
        }
        ops
    }

    pub fn validate_runtime_document(
        document: &crate::SignalSpaceDocument,
    ) -> Result<(), crate::ValidationError> {
        validate_document(document)
    }

    fn node_wire_id(idx: usize) -> u64 {
        2 + idx as u64
    }

    fn event_wire_id(node_count: usize, idx: usize) -> u64 {
        2 + node_count as u64 + idx as u64
    }

    fn node_payload<T: serde::Serialize>(id: u64, type_tag: &str, value: &T) -> NodeSnapshot {
        NodeSnapshot::payload(NodeId(id), type_tag, to_payload(value))
    }

    fn to_ipc_value<T: serde::Serialize>(value: &T) -> IpcValue {
        IpcValue::Inline(to_payload(value))
    }

    fn to_payload<T: serde::Serialize>(value: &T) -> Vec<u8> {
        serde_json::to_vec(value).expect("Signal Space mirror payload serializes")
    }

    pub fn timeline_by_class(graph: &SignalGraph, state_class: StateClass) -> Vec<&TimelineEvent> {
        graph
            .timeline
            .iter()
            .filter(|event| event.state_class == state_class)
            .collect()
    }
}
