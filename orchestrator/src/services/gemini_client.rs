//! Klien Gemini untuk ekstraksi tesis.
//! - `APP_MODE=mock`: parser deterministik untuk tes.
//! - `APP_MODE=real`: panggilan nyata (butuh `GEMINI_API_KEY` + `GEMINI_MODEL`).
//! Skema JSON hasil ekstraksi: `{"ticker": "...", "metrics": [{"metric_name": "...", "desired_direction": "increase|decrease|any"}]}`.
//! Daftar metric final & arahan arah divalidasi di Rust (LLM tidak sumber angka/aturan).

use serde::Deserialize;

use crate::config::{AppMode, Config};
use crate::errors::AppError;
use crate::metrics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedMetric {
    pub metric_name: String,
    pub desired_direction: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedThesis {
    pub ticker: String,
    pub metrics: Vec<ExtractedMetric>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExtracted {
    ticker: String,
    metrics: Vec<RawMetric>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMetric {
    metric_name: String,
    #[serde(default = "default_direction")]
    desired_direction: String,
}

fn default_direction() -> String {
    "any".into()
}

#[derive(Clone)]
pub struct GeminiClient {
    config: Config,
    http: reqwest::Client,
}

impl GeminiClient {
    pub fn new(config: &Config) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config: config.clone(),
            http,
        }
    }

    pub async fn extract_thesis(&self, text: &str) -> Result<ExtractedThesis, AppError> {
        match self.config.app_mode {
            AppMode::Mock => mock_extract(text),
            AppMode::Real => self.real_extract(text).await,
        }
    }

    async fn real_extract(&self, text: &str) -> Result<ExtractedThesis, AppError> {
        let key = self
            .config
            .gemini_api_key
            .as_deref()
            .ok_or_else(|| AppError::External("GEMINI_API_KEY belum diset".into()))?;
        let model = self
            .config
            .gemini_model
            .as_deref()
            .ok_or_else(|| AppError::External("GEMINI_MODEL belum diset".into()))?;

        let prompt = format!(
            "Anda adalah agen ekstraksi tesis pemantauan saham. Dari kalimat pengguna, keluarkan HANYA JSON valid dengan schema: \
             {{\"ticker\": \"<4 huruf kapital>\", \"metrics\": [{{\"metric_name\": \"<salah satu dari registry>\", \"desired_direction\": \"increase|decrease|any\"}}]}}. \
             Registry metric: {}. Maksimal 3 metric. Jangan menulis teks lain di luar JSON.\n\nKalimat pengguna: {}",
            metrics::metric_registry().join(", "),
            text
        );

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
        );
        let body = serde_json::json!({
            "contents": [{"parts": [{"text": prompt}]}],
            "generationConfig": {"responseMimeType": "application/json"}
        });
        let res = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::External(format!("gagal menghubungi Gemini: {e}")))?;
        let status = res.status();
        let json: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
        if !status.is_success() {
            return Err(AppError::External(format!("Gemini error {status}")));
        }
        let text = json
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::External("respons Gemini tanpa teks".into()))?;
        parse_extracted(text)
    }
}

/// Parse output Gemini/LLM: toleran terhadap code fence, tapi ketat pada schema.
pub fn parse_extracted(raw: &str) -> Result<ExtractedThesis, AppError> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let parsed: RawExtracted = serde_json::from_str(cleaned)
        .map_err(|_| AppError::BadRequest("output extractor bukan JSON schema tesis".into()))?;
    let extracted = ExtractedThesis {
        ticker: parsed.ticker,
        metrics: parsed
            .metrics
            .into_iter()
            .map(|m| ExtractedMetric {
                metric_name: m.metric_name,
                desired_direction: m.desired_direction,
            })
            .collect(),
    };
    validate_extracted(&extracted).map_err(AppError::BadRequest)?;
    Ok(extracted)
}

/// Validasi ketat hasil ekstraksi — LLM tidak boleh memutuskan aturan.
pub fn validate_extracted(t: &ExtractedThesis) -> Result<(), String> {
    if t.ticker.len() != 4
        || !t
            .ticker
            .chars()
            .all(|c| c.is_ascii_uppercase() && c.is_ascii_alphabetic())
    {
        return Err("ticker tidak valid".into());
    }
    if t.metrics.is_empty() {
        return Err("minimal satu metric".into());
    }
    if t.metrics.len() > 3 {
        return Err("maksimal 3 metric per tesis".into());
    }
    for m in &t.metrics {
        if !metrics::is_allowed(&m.metric_name) {
            return Err(format!("metric '{}' tidak ada di registry", m.metric_name));
        }
        if !matches!(
            m.desired_direction.as_str(),
            "increase" | "decrease" | "any"
        ) {
            return Err(format!(
                "desired_direction tidak valid: {}",
                m.desired_direction
            ));
        }
    }
    Ok(())
}

/// Ekstraktor mock deterministik untuk `APP_MODE=mock` dan pengujian.
pub fn mock_extract(text: &str) -> Result<ExtractedThesis, AppError> {
    let lower = text.to_lowercase();

    // Ticker: token 4 huruf kapital (case-insensitive di sini karena kita normalisasi).
    let mut ticker: Option<String> = None;
    for token in text.split(|c: char| !c.is_ascii_alphabetic()) {
        if token.len() == 4 && token.chars().all(|c| c.is_ascii_uppercase()) {
            ticker = Some(token.to_string());
            break;
        }
    }
    // Fallback mock: toleransi input lowercase seperti "bbca".
    if ticker.is_none() {
        for token in text.split(|c: char| !c.is_ascii_alphabetic()) {
            if token.len() == 4 && token.chars().all(|c| c.is_ascii_alphabetic()) {
                let upper = token.to_uppercase();
                // hanya terima bila muncul sebagai kode saham umum di teks (hindari kata biasa)
                if ["BBCA", "BBRI", "TLKM", "GGRM", "ASII", "UNVR", "TEST"]
                    .contains(&upper.as_str())
                {
                    ticker = Some(upper);
                    break;
                }
            }
        }
    }

    let mut found: Vec<ExtractedMetric> = Vec::new();
    for metric in metrics::metric_registry() {
        if lower.contains(&metric.to_lowercase()) {
            let direction = if lower.contains("turun") || lower.contains("decrease") {
                "decrease"
            } else if lower.contains("naik")
                || lower.contains("increase")
                || lower.contains("tumbuh")
                || lower.contains("mambaik")
                || lower.contains("membaik")
            {
                "increase"
            } else {
                "any"
            };
            found.push(ExtractedMetric {
                metric_name: metric,
                desired_direction: direction.into(),
            });
        }
    }

    let Some(ticker) = ticker else {
        return Err(AppError::BadRequest(
            "ticker tidak ditemukan; sebutkan kode saham 4 huruf".into(),
        ));
    };
    if found.is_empty() {
        return Err(AppError::BadRequest(
            "metric tidak dikenali; sebutkan metric dari registry".into(),
        ));
    }
    let extracted = ExtractedThesis {
        ticker,
        metrics: found,
    };
    validate_extracted(&extracted).map_err(AppError::BadRequest)?;
    Ok(extracted)
}
