use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Exact cosine retrieval is the default. HNSW requires an explicit opt-in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VectorIndex {
    #[default]
    Cosine,
    Hnsw,
}

impl std::str::FromStr for VectorIndex {
    type Err = crate::MemoryError;

    fn from_str(value: &str) -> crate::Result<Self> {
        match value {
            "cosine" => Ok(Self::Cosine),
            "hnsw" => Ok(Self::Hnsw),
            _ => Err(crate::MemoryError::InvalidInput(
                "vector index must be cosine or hnsw".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Preference,
    Fact,
    Decision,
    Procedure,
    ErrorFix,
    TaskResult,
    Entity,
    ConversationSummary,
}

impl MemoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Procedure => "procedure",
            Self::ErrorFix => "error_fix",
            Self::TaskResult => "task_result",
            Self::Entity => "entity",
            Self::ConversationSummary => "conversation_summary",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "preference" => Self::Preference,
            "fact" => Self::Fact,
            "decision" => Self::Decision,
            "procedure" => Self::Procedure,
            "error_fix" => Self::ErrorFix,
            "task_result" => Self::TaskResult,
            "entity" => Self::Entity,
            "conversation_summary" => Self::ConversationSummary,
            _ => return None,
        })
    }
}

/// High-level taxonomy for durable memories.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Profile,
    Preferences,
    Entities,
    Events,
    Identity,
    Soul,
    Cases,
    Trajectories,
    Experiences,
    /// Consolidated, evidence-backed beliefs derived from multiple memories.
    Observations,
    /// Standing synthesized knowledge pages / mental models.
    Knowledge,
    /// Prospective intentions with an expiry.
    Intentions,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Preferences => "preferences",
            Self::Entities => "entities",
            Self::Events => "events",
            Self::Identity => "identity",
            Self::Soul => "soul",
            Self::Cases => "cases",
            Self::Trajectories => "trajectories",
            Self::Experiences => "experiences",
            Self::Observations => "observations",
            Self::Knowledge => "knowledge",
            Self::Intentions => "intentions",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "profile" => Self::Profile,
            "preferences" => Self::Preferences,
            "entities" => Self::Entities,
            "events" => Self::Events,
            "identity" => Self::Identity,
            "soul" => Self::Soul,
            "cases" => Self::Cases,
            "trajectories" => Self::Trajectories,
            "experiences" => Self::Experiences,
            "observations" => Self::Observations,
            "knowledge" => Self::Knowledge,
            "intentions" => Self::Intentions,
            _ => return None,
        })
    }

    pub fn inferred_from_kind(kind: &MemoryKind) -> Self {
        match kind {
            MemoryKind::Preference => Self::Preferences,
            MemoryKind::Entity | MemoryKind::Fact => Self::Entities,
            MemoryKind::Procedure | MemoryKind::ErrorFix => Self::Experiences,
            MemoryKind::Decision | MemoryKind::TaskResult | MemoryKind::ConversationSummary => {
                Self::Events
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextLayer {
    /// Short abstract used for retrieval and quick relevance checks.
    Abstract,
    /// Navigable overview used before loading full detail.
    #[default]
    Overview,
    /// Complete stored content, loaded only when explicitly requested.
    Detail,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(rename = "commit", skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreInput {
    pub content: String,
    pub scope: String,
    pub kind: MemoryKind,
    /// High-level memory collection. Inferred from `kind` when omitted.
    #[serde(default)]
    pub memory_type: Option<MemoryType>,
    /// L0: concise retrieval abstract. Generated deterministically when omitted.
    #[serde(default, rename = "abstract", alias = "abstract_text")]
    pub abstract_text: Option<String>,
    /// L1: broader overview used to decide whether L2 detail is needed.
    #[serde(default)]
    pub overview: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub proof_count: Option<i64>,
    #[serde(default)]
    pub knowledge_path: Option<String>,
    #[serde(default)]
    pub chunks: Vec<String>,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default = "default_importance")]
    pub importance: f64,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub valid_from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub valid_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub dedupe_key: Option<String>,
    #[serde(default)]
    pub supersedes: Option<String>,
}

fn default_importance() -> f64 {
    0.5
}
fn default_confidence() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateInput {
    pub id: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub kind: Option<MemoryKind>,
    #[serde(default)]
    pub memory_type: Option<MemoryType>,
    #[serde(default, rename = "abstract", alias = "abstract_text")]
    pub abstract_text: Option<String>,
    #[serde(default)]
    pub overview: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub entities: Option<Vec<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub proof_count: Option<i64>,
    #[serde(default)]
    pub knowledge_path: Option<String>,
    #[serde(default)]
    pub chunks: Option<Vec<String>>,
    #[serde(default)]
    pub private: Option<bool>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub importance: Option<f64>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub valid_from: Option<DateTime<Utc>>,
    #[serde(default)]
    pub valid_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub dedupe_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchInput {
    pub query: String,
    /// Exact allowed scopes. No implicit global or cross-repository search occurs.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Convenience for a single exact scope.
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub kinds: Vec<MemoryKind>,
    #[serde(default)]
    pub memory_types: Vec<MemoryType>,
    /// Tag filter: only memories containing all of these tags are returned.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Only memories created at or after this timestamp.
    #[serde(default)]
    pub created_after: Option<DateTime<Utc>>,
    /// Only memories created at or before this timestamp.
    #[serde(default)]
    pub created_before: Option<DateTime<Utc>>,
    /// Bi-temporal: view memory as it existed at this time.
    #[serde(default)]
    pub as_of: Option<DateTime<Utc>>,
    /// Optional natural-language time reference resolved into created_after/created_before.
    #[serde(default)]
    pub time_query: Option<String>,
    /// Expand results with one-hop typed links (graph traversal).
    #[serde(default)]
    pub expand_links: bool,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    8
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Memory {
    pub id: String,
    pub scope: String,
    pub kind: MemoryKind,
    pub memory_type: MemoryType,
    /// L0 bounded abstract used for vector retrieval.
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    /// L1 overview used for navigation and bounded context.
    pub overview: String,
    /// L2 complete detail.
    pub content: String,
    pub summary: Option<String>,
    pub entities: Vec<String>,
    pub tags: Vec<String>,
    pub source: Option<Source>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub importance: f64,
    pub confidence: f64,
    pub content_hash: String,
    pub dedupe_key: Option<String>,
    pub supersedes: Option<String>,
    pub embedding_model: String,
    pub embedding_version: String,
    pub redacted: bool,
    /// Number of supporting observations/evidence items (used for observations).
    pub proof_count: i64,
    /// Optional knowledge-page path for synthesized knowledge memories.
    pub knowledge_path: Option<String>,
    /// Optional raw source chunks for expanded context.
    pub chunks: Vec<String>,
    pub access_count: i64,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub private: bool,
    pub contradiction_of: Option<String>,
    pub compacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemoryPreview {
    pub id: String,
    pub scope: String,
    pub kind: MemoryKind,
    pub memory_type: MemoryType,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub entities: Vec<String>,
    pub tags: Vec<String>,
    pub source: Option<Source>,
    pub updated_at: DateTime<Utc>,
    pub importance: f64,
    pub confidence: f64,
}

impl From<&Memory> for MemoryPreview {
    fn from(memory: &Memory) -> Self {
        Self {
            id: memory.id.clone(),
            scope: memory.scope.clone(),
            kind: memory.kind.clone(),
            memory_type: memory.memory_type.clone(),
            abstract_text: memory.abstract_text.clone(),
            entities: memory.entities.clone(),
            tags: memory.tags.clone(),
            source: memory.source.clone(),
            updated_at: memory.updated_at,
            importance: memory.importance,
            confidence: memory.confidence,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchHit {
    /// L0 preview only. Use `span_overview`, then `span_get` for more detail.
    pub memory: MemoryPreview,
    pub score: f64,
    pub citation: String,
    pub matched_by: Vec<String>,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResults {
    pub results: Vec<SearchHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreResult {
    pub memory: Memory,
    pub created: bool,
    pub deduplicated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForgetResult {
    pub id: String,
    pub hard_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LinkInput {
    pub from_id: String,
    pub to_id: String,
    pub relation: String,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemoryLink {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub relation: String,
    #[schemars(schema_with = "unrestricted_metadata_schema")]
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

fn unrestricted_metadata_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    // An empty object accepts any JSON value, including on MCP clients that reject boolean schemas.
    schemars::json_schema!({})
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetResult {
    pub memory: Memory,
    pub links: Vec<MemoryLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OverviewResult {
    pub id: String,
    pub scope: String,
    pub kind: MemoryKind,
    pub memory_type: MemoryType,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub overview: String,
    pub source: Option<Source>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextInput {
    pub query: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub kinds: Vec<MemoryKind>,
    #[serde(default)]
    pub memory_types: Vec<MemoryType>,
    /// Tag filter: only memories containing all of these tags are returned.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Only memories created at or after this timestamp.
    #[serde(default)]
    pub created_after: Option<DateTime<Utc>>,
    /// Only memories created at or before this timestamp.
    #[serde(default)]
    pub created_before: Option<DateTime<Utc>>,
    /// Bi-temporal: view memory as it existed at this time.
    #[serde(default)]
    pub as_of: Option<DateTime<Utc>>,
    /// Optional natural-language time reference resolved into created_after/created_before.
    #[serde(default)]
    pub time_query: Option<String>,
    /// Expand results with one-hop typed links (graph traversal).
    #[serde(default)]
    pub expand_links: bool,
    /// Highest context layer to include. Defaults to the bounded L1 overview.
    #[serde(default)]
    pub layer: ContextLayer,
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
}

fn default_token_budget() -> usize {
    1200
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReflectResult {
    pub packet: ContextPacket,
    pub reflection_prompt: String,
    pub conclusions: Vec<CitedClaim>,
    pub conflicts: Vec<(String, String)>,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CitedClaim {
    pub text: String,
    pub memory_id: String,
    pub citation: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContextPacket {
    pub text: String,
    pub citations: Vec<String>,
    pub memories: Vec<SearchHit>,
    pub estimated_tokens: usize,
    pub layer: ContextLayer,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CodeRevision {
    pub id: String,
    pub scope: String,
    pub path: String,
    pub branch: Option<String>,
    pub user_id: Option<String>,
    pub workspace_id: Option<String>,
    pub revision_kind: String,
    pub diff: String,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecordRevisionInput {
    pub scope: String,
    pub path: String,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default = "default_revision_kind")]
    pub revision_kind: String,
    pub diff: String,
    #[serde(default)]
    pub summary: Option<String>,
}

fn default_revision_kind() -> String {
    "edit".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RevisionsInput {
    pub scope: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgePagesInput {
    pub scope: String,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TimelineInput {
    pub scope: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub anchor_id: Option<String>,
    #[serde(default = "default_before")]
    pub before: usize,
    #[serde(default = "default_after")]
    pub after: usize,
    #[serde(default)]
    pub as_of: Option<DateTime<Utc>>,
}

fn default_before() -> usize {
    5
}
fn default_after() -> usize {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TouchInput {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HandoffCreateInput {
    pub scope: String,
    pub owner: String,
    pub content: String,
    #[serde(default)]
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HandoffClaimInput {
    pub id: String,
    pub claimant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HandoffListInput {
    pub scope: String,
    #[serde(default)]
    pub include_claimed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Handoff {
    pub id: String,
    pub scope: String,
    pub owner: String,
    pub claimed_by: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompactInput {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaintenanceInput {
    pub scope: String,
    #[serde(default)]
    pub apply: bool,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MaintenanceResult {
    pub conflicts: Vec<(String, String)>,
    pub expired_ids: Vec<String>,
    pub stale_derived_ids: Vec<String>,
    pub applied: bool,
    pub refreshed_page_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CaptureEvent {
    pub event_id: String,
    pub scope: String,
    pub event: String,
    /// Only explicitly approved durable text, never a raw transcript.
    pub content: String,
    #[serde(default)]
    pub successful: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KnowledgePageStatus {
    pub memory: MemoryPreview,
    pub stale: bool,
    pub source_query: Option<String>,
    pub refreshed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContradictionInput {
    pub scope: String,
    #[serde(default)]
    pub threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportInput {
    pub scope: String,
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportInput {
    pub scope: String,
    pub path: String,
    #[serde(default)]
    pub memory_type: Option<MemoryType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProfileInput {
    pub user: String,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditEntry {
    pub id: String,
    pub actor: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConsolidateInput {
    pub scopes: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceStatus {
    pub source_ids: Vec<String>,
    pub proof_count: usize,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatchInput {
    pub scope: String,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HistoryInput {
    pub scope: String,
    pub id: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HistoryResult {
    pub revisions: Vec<Memory>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefreshInput {
    pub scope: String,
    pub path: String,
    pub query: String,
    /// Preview is the default. Commit only after reviewing the evidence.
    #[serde(default)]
    pub apply: bool,
    /// Apply only the exact source snapshot returned by a preview.
    #[serde(default)]
    pub expected_source_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefreshResult {
    pub content: String,
    pub source_ids: Vec<String>,
    pub outcome: String,
    pub memory: Option<Memory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchDiagnostics {
    pub results: Vec<SearchHit>,
    pub eligible_count: usize,
    pub elapsed_micros: u128,
    pub scoring: String,
    pub channel_counts: std::collections::BTreeMap<String, usize>,
    pub reranker_status: String,
    pub stage_micros: std::collections::BTreeMap<String, u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoreStatus {
    pub schema_version: i64,
    pub active_memories: u64,
    pub deleted_memories: u64,
    pub links: u64,
    pub integrity: String,
    pub embedding_model: String,
    pub embedding_version: String,
    pub embedding_dimensions: usize,
    pub stored_embeddings: Vec<StoredEmbeddingCount>,
    pub incompatible_embeddings: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StoredEmbeddingCount {
    pub model: String,
    pub version: String,
    /// None when the stored value is not a nonempty sequence of f32 values.
    pub dimensions: Option<usize>,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReindexResult {
    pub reindexed: usize,
    pub embedding_model: String,
    pub embedding_version: String,
}
