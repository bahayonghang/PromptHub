//! DTOs for the local prompt execution and evaluation loop.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::PromptMessage;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionProfileInput {
    pub profile_id: Option<String>,
    pub name: String,
    pub provider: String,
    pub endpoint: Option<String>,
    pub model: String,
    #[serde(default = "empty_object")]
    pub parameters: Value,
    pub credential: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionProfileRevision {
    pub id: String,
    pub profile_id: String,
    pub revision: i64,
    pub name: String,
    pub provider: String,
    pub endpoint: Option<String>,
    pub model: String,
    pub parameters: Value,
    pub has_credential: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPrompt {
    pub prompt_revision_id: String,
    pub messages: Vec<PromptMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptRunInput {
    pub prompt_revision_id: String,
    pub profile_revision_id: String,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    pub test_case_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptRun {
    pub id: String,
    pub prompt_revision_id: String,
    pub profile_revision_id: String,
    pub test_case_id: Option<String>,
    pub inputs: BTreeMap<String, String>,
    pub rendered_messages: Vec<PromptMessage>,
    pub output: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub usage: Option<Value>,
    pub cache_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCaseInput {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    pub expected_output: Option<String>,
    #[serde(default = "empty_object")]
    pub annotations: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSetInput {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub cases: Vec<TestCaseInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub inputs: BTreeMap<String, String>,
    pub expected_output: Option<String>,
    pub annotations: Value,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestSet {
    pub id: String,
    pub name: String,
    pub cases: Vec<TestCase>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatorInput {
    pub name: String,
    pub kind: String,
    #[serde(default = "empty_object")]
    pub config: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatorConfig {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub config: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationResult {
    pub evaluator_id: String,
    pub kind: String,
    pub passed: Option<bool>,
    pub score: Option<f64>,
    pub skipped: bool,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationMatrixInput {
    pub test_set_id: String,
    pub prompt_revision_ids: Vec<String>,
    pub profile_revision_ids: Vec<String>,
    pub evaluator_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationRun {
    pub id: String,
    pub test_set_id: String,
    pub prompt_revision_ids: Vec<String>,
    pub profile_revision_ids: Vec<String>,
    pub evaluator_ids: Vec<String>,
    pub status: String,
    pub total_cells: i64,
    pub completed_cells: i64,
    pub failed_cells: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub runtime_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationCell {
    pub id: String,
    pub evaluation_run_id: String,
    pub prompt_revision_id: String,
    pub profile_revision_id: String,
    pub test_case_id: String,
    pub prompt_run_id: Option<String>,
    pub status: String,
    pub cache_hit: bool,
    pub results: Vec<EvaluationResult>,
    pub error: Option<String>,
    pub cache_key: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationRunDetail {
    pub run: EvaluationRun,
    pub cells: Vec<EvaluationCell>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptLabel {
    pub prompt_id: String,
    pub label: String,
    pub prompt_revision_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptLabelHistory {
    pub id: String,
    pub prompt_id: String,
    pub label: String,
    pub from_revision_id: Option<String>,
    pub to_revision_id: String,
    pub action: String,
    pub created_at: String,
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}
