use serde::{Deserialize, Serialize};

/// Official disclaimer — source of truth in CONTRACTS.md. Do not change before team decision.
pub const DISCLAIMER_RESMI: &str = "Disclaimer: Informasi ini hanya bersifat edukasi & pemantauan data historis. TIDAK memberikan saran investasi (beli/jual/tahan) maupun eksekusi transaksi.";

/// Payload alert internal sesuai schema CONTRACTS.md (additionalProperties dilarang).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhatsAppAlertPayload {
    pub recipient_number: String,
    pub header: String,
    pub body: String,
    pub disclaimer: String,
}

impl WhatsAppAlertPayload {
    /// Validasi sesuai kontrak: pola nomor, panjang header/body, disclaimer persis konstanta.
    pub fn validate(&self) -> Result<(), String> {
        let ok_number = self.recipient_number.len() >= 10
            && self.recipient_number.len() <= 15
            && self.recipient_number.chars().all(|c| c.is_ascii_digit())
            && self
                .recipient_number
                .chars()
                .next()
                .is_some_and(|c| c != '0');
        if !ok_number {
            return Err("recipient_number harus 10–15 digit, tanpa '+', digit pertama 1–9".into());
        }
        if self.header.len() > 100 {
            return Err("header maksimal 100 karakter".into());
        }
        if self.body.len() > 2048 {
            return Err("body maksimal 2048 karakter".into());
        }
        if self.header.trim().is_empty() || self.body.trim().is_empty() {
            return Err("header/body tidak boleh kosong".into());
        }
        if self.disclaimer != DISCLAIMER_RESMI {
            return Err("disclaimer harus sama persis dengan konstanta CONTRACTS.md".into());
        }
        Ok(())
    }
}

/// Hasil pengiriman — jujur: mock TIDAK pernah berstatus "sent" sungguhan.
#[derive(Debug, Clone, Serialize)]
pub struct SendOutcome {
    /// `mock_sent` | `sent` | `blocked_by_policy_conflict` | `failed`
    pub status: String,
    pub message_id: Option<String>,
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> WhatsAppAlertPayload {
        WhatsAppAlertPayload {
            recipient_number: "6281234567890".into(),
            header: "BBCA · Net Profit Margin".into(),
            body: "Nilai terbaru: 46.85.".into(),
            disclaimer: DISCLAIMER_RESMI.into(),
        }
    }

    #[test]
    fn payload_valid() {
        assert!(valid().validate().is_ok());
    }

    #[test]
    fn nomor_dengan_plus_ditolak() {
        let mut p = valid();
        p.recipient_number = "+6281234567890".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn header_terlalu_panjang_ditolak() {
        let mut p = valid();
        p.header = "x".repeat(101);
        assert!(p.validate().is_err());
    }

    #[test]
    fn disclaimer_berbeda_ditolak() {
        let mut p = valid();
        p.disclaimer = "Disclaimer versi lain.".into();
        assert!(p.validate().is_err());
    }
}
