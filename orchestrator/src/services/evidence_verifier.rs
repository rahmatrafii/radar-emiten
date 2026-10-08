//! Zero-Hallucination Verifier (D-03) — milik Dian.
//!
//! Memverifikasi 1:1 bahwa angka `current_value` / `previous_value` pada
//! `Finding` identik dengan snapshot tersimpan di PostgreSQL (sumber Sectors API v2).
//!
//! Referensi:
//! - `docs/CONTRACTS.md` — schema `ComplianceVerificationResult`
//! - `docs/IDX_Sentinel_Rafi_Agent_Execution_Runbook.md` — Bagian 13.1
//!
//! Catatan: modul ini mengekstrak logic Gate 2 yang sebelumnya inline di
//! `routes::findings::accept_finding` menjadi service reusable + testable.
//! Tidak ada perubahan perilaku verifikasi.

use sqlx::PgPool;

use crate::models::finding::Finding;
use crate::repositories::snapshots;
use crate::services::compliance_service;

/// Hasil verifikasi evidensi angka saja.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
    pub evidence_verified: bool,
    pub rejection_reason: Option<String>,
}

/// Hasil gabungan compliance + evidence, sesuai schema `ComplianceVerificationResult`
/// di `docs/CONTRACTS.md`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ComplianceVerificationResult {
    pub finding_id: String,
    pub is_compliant: bool,
    pub prohibited_words_detected: Vec<String>,
    pub evidence_verified: bool,
    pub rejection_reason: Option<String>,
}

/// Verifikasi 1:1 angka Finding terhadap snapshot database.
///
/// - `current_value` harus cocok dengan snapshot periode + source yang sama.
/// - `previous_value` harus cocok dengan snapshot lain (periode berbeda).
///
/// Mengembalikan `Ok(VerificationResult)` untuk hasil lolos/gagal verifikasi,
/// dan `Err(sqlx::Error)` hanya untuk kegagalan database itu sendiri.
pub async fn verify_evidence(
    pool: &PgPool,
    finding: &Finding,
) -> Result<VerificationResult, sqlx::Error> {
    let current = match crate::validation::to_f64(&finding.current_value) {
        Some(v) => v,
        None => {
            return Ok(VerificationResult {
                evidence_verified: false,
                rejection_reason: Some("current_value harus numerik yang valid".to_string()),
            });
        }
    };

    let previous = match crate::validation::to_f64(&finding.previous_value) {
        Some(v) => v,
        None => {
            return Ok(VerificationResult {
                evidence_verified: false,
                rejection_reason: Some("previous_value harus numerik yang valid".to_string()),
            });
        }
    };

    let current_ok = snapshots::value_exists(
        pool,
        &finding.ticker,
        &finding.metric_name,
        &finding.period,
        &finding.source,
        current,
    )
    .await?;

    let previous_ok = snapshots::previous_exists(
        pool,
        &finding.ticker,
        &finding.metric_name,
        previous,
        &finding.period,
    )
    .await?;

    if current_ok && previous_ok {
        Ok(VerificationResult {
            evidence_verified: true,
            rejection_reason: None,
        })
    } else {
        let reason = if !current_ok && !previous_ok {
            "current_value/previous_value tidak cocok dengan snapshot tersimpan".to_string()
        } else if !current_ok {
            "current_value tidak cocok dengan snapshot tersimpan".to_string()
        } else {
            "previous_value tidak cocok dengan snapshot tersimpan".to_string()
        };
        Ok(VerificationResult {
            evidence_verified: false,
            rejection_reason: Some(reason),
        })
    }
}

/// Jalur lengkap verifikasi: compliance check (D-02) + evidence verification.
///
/// - `is_compliant` true hanya bila teks bersih DAN angka terverifikasi.
/// - `rejection_reason` memprioritaskan alasan compliance bila keduanya gagal.
pub async fn verify_finding(
    pool: &PgPool,
    finding: &Finding,
    finding_id: Option<i64>,
) -> Result<ComplianceVerificationResult, sqlx::Error> {
    let compliance = compliance_service::check_compliance(&finding.finding_summary);
    let evidence = verify_evidence(pool, finding).await?;

    let rejection_reason = if !compliance.is_compliant {
        Some(format!(
            "kata terlarang terdeteksi: {}",
            compliance.prohibited_words_detected.join(", ")
        ))
    } else {
        evidence.rejection_reason.clone()
    };

    Ok(ComplianceVerificationResult {
        finding_id: finding_id.map(|id| id.to_string()).unwrap_or_default(),
        is_compliant: compliance.is_compliant && evidence.evidence_verified,
        prohibited_words_detected: compliance.prohibited_words_detected,
        evidence_verified: evidence.evidence_verified,
        rejection_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_result_valid_bentuk_benar() {
        let r = VerificationResult {
            evidence_verified: true,
            rejection_reason: None,
        };
        assert!(r.evidence_verified);
        assert!(r.rejection_reason.is_none());
    }

    #[test]
    fn verification_result_invalid_membawa_alasan() {
        let r = VerificationResult {
            evidence_verified: false,
            rejection_reason: Some("current_value tidak cocok".to_string()),
        };
        assert!(!r.evidence_verified);
        assert!(r.rejection_reason.is_some());
    }

    #[test]
    fn compliance_verification_result_serialisasi_sesuai_kontrak() {
        let r = ComplianceVerificationResult {
            finding_id: "123".to_string(),
            is_compliant: true,
            prohibited_words_detected: vec![],
            evidence_verified: true,
            rejection_reason: None,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["finding_id"], serde_json::json!("123"));
        assert_eq!(v["is_compliant"], serde_json::json!(true));
        assert_eq!(v["evidence_verified"], serde_json::json!(true));
        assert!(v.get("prohibited_words_detected").is_some());
        assert!(v.get("rejection_reason").is_some());
    }

    #[test]
    fn compliance_verification_result_rejected_berseri_dengan_benar() {
        let r = ComplianceVerificationResult {
            finding_id: String::new(),
            is_compliant: false,
            prohibited_words_detected: vec!["beli".to_string()],
            evidence_verified: false,
            rejection_reason: Some("kata terlarang terdeteksi: beli".to_string()),
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("beli"));
        assert!(s.contains("\"is_compliant\":false"));
    }
}
