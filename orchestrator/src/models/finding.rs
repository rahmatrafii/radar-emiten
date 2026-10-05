use serde::{Deserialize, Serialize};

/// Payload eksternal resmi per CONTRACTS.md (additionalProperties dilarang).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub ticker: String,
    pub subsector: String,
    pub metric_name: String,
    pub current_value: serde_json::Value,
    pub previous_value: serde_json::Value,
    pub period: String,
    pub source: String,
    pub observed_at: String,
    pub confidence_score: f64,
    pub finding_summary: String,
}
