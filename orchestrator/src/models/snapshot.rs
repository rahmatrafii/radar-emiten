use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Snapshot {
    pub id: i64,
    pub ticker: String,
    pub metric_name: String,
    pub period: String,
    pub value: f64,
    pub source: String,
    pub diambil_pada: DateTime<Utc>,
    pub subsector: Option<String>,
}
