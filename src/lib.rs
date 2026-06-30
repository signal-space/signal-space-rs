use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const SPEC_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeFamily {
    Source,
    Transform,
    Memory,
    Decision,
    Gate,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSpaceDocument {
    pub schema_version: String,
    pub graph: SignalGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub provenance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeState {
    pub summary: String,
    #[serde(default)]
    pub fields: Vec<StateField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateField {
    pub id: String,
    pub state_class: StateClass,
    pub writable: bool,
    #[serde(default)]
    pub derived: bool,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authority {
    pub default: AuthorityLevel,
    #[serde(default)]
    pub by_intent: std::collections::BTreeMap<String, AuthorityLevel>,
    pub owner: Option<String>,
    pub boundary: Option<String>,
    #[serde(default)]
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentModule {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    pub capability: Option<String>,
    #[serde(default)]
    pub intents: Vec<IntentSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    if document.schema_version != SPEC_VERSION {
        return Err(ValidationError::new(format!(
            "unsupported spec version: {}",
            document.schema_version
        )));
    }

    let mut node_ids = HashSet::new();
    for node in &document.graph.nodes {
        if !node_ids.insert(node.id.as_str()) {
            return Err(ValidationError::new(format!("duplicate node id: {}", node.id)));
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
    }

    let mut edge_ids = HashSet::new();
    for edge in &document.graph.edges {
        if !edge_ids.insert(edge.id.as_str()) {
            return Err(ValidationError::new(format!("duplicate edge id: {}", edge.id)));
        }
        if !node_ids.contains(edge.from_node.as_str()) || !node_ids.contains(edge.to_node.as_str())
        {
            return Err(ValidationError::new(format!(
                "edge has unknown endpoint: {}",
                edge.id
            )));
        }
    }

    Ok(())
}

pub fn round_trip(document: &SignalSpaceDocument) -> Result<SignalSpaceDocument, serde_json::Error> {
    serde_json::from_value(serde_json::to_value(document)?)
}
