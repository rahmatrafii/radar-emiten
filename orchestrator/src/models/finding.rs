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

/// Baris findings untuk kebutuhan list/dashboard (kolom internal tidak jadi payload Finding).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct FindingRecord {
    pub id: i64,
    pub ticker: Option<String>,
    pub subsector: Option<String>,
    pub metric_name: Option<String>,
    pub current_value: Option<f64>,
    pub previous_value: Option<f64>,
    pub period: Option<String>,
    pub source: Option<String>,
    pub observed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub confidence_score: Option<f64>,
    pub finding_summary: Option<String>,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub dibuat_pada: chrono::DateTime<chrono::Utc>,
}
