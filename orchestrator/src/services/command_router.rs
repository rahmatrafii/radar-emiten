//! Command router WhatsApp -> workflow tesis & perintah pengguna.
//! Balasan dibangun dengan template kode (bukan LLM bebas). Pengiriman lewat WhatsAppClient
//! dilakukan oleh pemanggil; fungsi ini menghasilkan teks balasan.

use crate::errors::AppError;
use crate::metrics;
use crate::repositories::{credits, findings, theses, users};
use crate::state::AppState;

const HELP: &str = "Perintah tersedia:\n\
/tesis - mulai input tesis\n\
YA - aktifkan tesis pending\n\
TIDAK - batalkan tesis pending\n\
/status - tesis aktif & status\n\
/bukti - data/finding terakhir\n\
/diam - heningkan alert proaktif\n\
/lanjut - aktifkan kembali alert\n\
/hapus <id_tesis> - hapus tesis\n\
/kredit - meter kredit (admin)\n\
/bantuan - daftar ini\n\n\
⚠️ *Disclaimer:* Ringkasan ini hanya bersifat edukasi & pemantauan data historis. TIDAK memberikan saran investasi (beli/jual/tahan) maupun eksekusi transaksi.";

fn normalized(number: &str) -> String {
    number.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Proses satu pesan text; kembalikan teks balasan untuk dikirim ke pengirim.
pub async fn route(
    state: &AppState,
    user_id: i64,
    from_number: &str,
    body: &str,
) -> Result<String, AppError> {
    let text = body.trim();
    let lower = text.to_lowercase();

    match lower.as_str() {
        "/bantuan" => return Ok(HELP.into()),

        "/tesis" => {
            return Ok("Kirim kalimat tesis Anda, contoh: \"BBCA net_profit_margin naik\"".into());
        }

        "ya" => {
            let Some((tes_id, _ticker)) = theses::latest_pending(&state.pool, user_id).await?
            else {
                return Ok("Tidak ada tesis yang menunggu konfirmasi.".into());
            };
            let active = theses::count_active(&state.pool, user_id).await?;
            if active >= 3 {
                return Ok(
                    "Batas maksimal 3 tesis aktif tercapai. Hapus dulu dengan /hapus <id>.".into(),
                );
            }
            theses::activate(&state.pool, tes_id).await?;
            return Ok(format!(
                "Tesis #{tes_id} diaktifkan. Cek /status untuk memantau."
            ));
        }

        "tidak" => {
            let cancelled = theses::cancel_pending(&state.pool, user_id).await?;
            return Ok(if cancelled {
                "Tesis pending dibatalkan.".into()
            } else {
                "Tidak ada tesis pending untuk dibatalkan.".into()
            });
        }

        "/status" => {
            let rows = theses::list_active(&state.pool, user_id).await?;
            if rows.is_empty() {
                return Ok("Belum ada tesis aktif.".into());
            }
            let mut out = String::from("Tesis aktif:\n");
            for (id, ticker, status) in rows {
                out.push_str(&format!("• #{id} {ticker} — {status}\n"));
            }
            return Ok(out.trim_end().into());
        }

        "/bukti" => {
            return match findings::latest_accepted_for_user(&state.pool, user_id).await? {
                Some(f) => {
                    let prev_str = f
                        .previous_value
                        .map(|v| format!("{v:.2}"))
                        .unwrap_or_else(|| "-".into());
                    let curr_str = f
                        .current_value
                        .map(|v| format!("{v:.2}"))
                        .unwrap_or_else(|| "-".into());
                    let conf_str = f
                        .confidence_score
                        .map(|v| format!("{:.0}%", v * 100.0))
                        .unwrap_or_else(|| "-".into());
                    Ok(format!(
                        "Bukti terakhir:\nTicker: {}\nMetric: {}\nPeriode: {}\nSebelumnya: {}\nSekarang: {}\nSumber: {}\nConfidence: {}",
                        f.ticker.unwrap_or_default(),
                        f.metric_name.unwrap_or_default(),
                        f.period.unwrap_or_default(),
                        prev_str,
                        curr_str,
                        f.source.unwrap_or_default(),
                        conf_str,
                    ))
                }
                None => Ok("Belum ada finding lolos untuk tesis Anda.".into()),
            };
        }

        "/diam" => {
            // Durasi belum disepakati tim: mute sampai /lanjut.
            let until = chrono::DateTime::parse_from_rfc3339("2099-12-31T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc);
            users::set_muted_until(&state.pool, user_id, Some(until)).await?;
            return Ok(
                "Alert proaktif dihentikan sampai /lanjut. (Durasi mute belum ditetapkan tim; menggunakan mode hening tak berwaktu.)"
                    .into(),
            );
        }

        "/lanjut" => {
            users::set_muted_until(&state.pool, user_id, None).await?;
            return Ok("Alert proaktif diaktifkan kembali.".into());
        }

        "/kredit" => {
            let admin = state
                .config
                .admin_phone
                .as_deref()
                .map(normalized)
                .unwrap_or_default();
            if admin.is_empty() || normalized(from_number) != admin {
                return Ok("Perintah /kredit hanya untuk admin.".into());
            }
            let used = credits::total_used(&state.pool).await?;
            let budget = state.config.sectors_credit_budget;
            return Ok(format!(
                "Meter kredit:\nBudget (konfigurasi): {budget}\nTerpakai: {used}\nSisa: {:.0}",
                (budget - used as f64).max(0.0)
            ));
        }

        _ => {}
    }

    if let Some(rest) = lower.strip_prefix("/hapus") {
        let arg = rest.trim();
        if arg.is_empty() {
            let rows = theses::list_active(&state.pool, user_id).await?;
            if rows.is_empty() {
                return Ok("Tidak ada tesis aktif untuk dihapus.".into());
            }
            let mut out = String::from("Pilih tesis yang dihapus dengan /hapus <id>:\n");
            for (id, ticker, status) in rows {
                out.push_str(&format!("• #{id} {ticker} — {status}\n"));
            }
            return Ok(out.trim_end().into());
        }
        let Ok(id) = arg.parse::<i64>() else {
            return Ok("Format: /hapus <id_tesis>".into());
        };
        return Ok(if theses::delete_by_id(&state.pool, user_id, id).await? {
            format!("Tesis #{id} dihapus.")
        } else {
            format!("Tesis #{id} tidak ditemukan milik Anda.")
        });
    }

    if lower.starts_with('/') {
        return Ok(format!("Perintah tidak dikenal: {text}\n\n{HELP}"));
    }

    // Kalimat biasa diperlakukan sebagai tesis.
    let extracted = match state.gemini.extract_thesis(text).await {
        Ok(t) => t,
        Err(AppError::BadRequest(reason)) => {
            return Ok(format!(
                "Saya belum memahami tesis Anda ({reason}).\nContoh: \"BBCA net_profit_margin naik\". \
                 Metric tersedia: {}",
                metrics::metric_registry().join(", ")
            ));
        }
        Err(e) => return Err(e),
    };

    let thesis_id = theses::insert_pending(&state.pool, user_id, text, &extracted.ticker).await?;
    for m in &extracted.metrics {
        theses::add_indicator(&state.pool, thesis_id, &m.metric_name, &m.desired_direction).await?;
    }

    let mut summary = format!(
        "Saya akan memantau tesis berikut:\nTicker: {}\n",
        extracted.ticker
    );
    for m in &extracted.metrics {
        summary.push_str(&format!(
            "Metric: {}\nArah: {}\n",
            m.metric_name, m.desired_direction
        ));
    }
    summary.push_str("\nBalas YA untuk mengaktifkan pemantauan, TIDAK untuk membatalkan.");
    Ok(summary)
}
