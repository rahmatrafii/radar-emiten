// src/sectors/client.rs — Sectors API v2 Client (inti)

use std::sync::Arc;

use reqwest::{Client, StatusCode};
use tracing::{debug, info, instrument, warn};

use crate::findings::{Finding, TingkatKeyakinan};
use crate::sectors::{
    cache::MemCache,
    credits::{cost, CreditTracker},
    error::{Result, SectorsError},
    models::{
        quarterly::{QuarterlyReport, QuarterlyReportResponse},
        screener::{ScreenerQuery, ScreenerResponse, ScreenerRow},
    },
};

/// Base URL Sectors API v2.
const BASE_URL: &str = "https://api.sectors.app/v2";

/// Environment variable yang menyimpan API key.
/// **JANGAN pernah hardcode API key di source code.**
const API_KEY_ENV: &str = "SECTORS_API_KEY";

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Konfigurasi SectorsClient.
#[derive(Debug, Clone)]
pub struct SectorsConfig {
    /// Batas kredit API per sesi/hari.
    pub credit_budget: u64,

    /// Apakah cache diaktifkan?
    pub cache_enabled: bool,

    /// Timeout HTTP per request (detik).
    pub timeout_secs: u64,
}

impl Default for SectorsConfig {
    fn default() -> Self {
        Self {
            credit_budget: 5_000,
            cache_enabled: true,
            timeout_secs: 30,
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client utama untuk Sectors API v2.
///
/// Aman digunakan secara konkuren — bungkus dalam `Arc` untuk berbagi antar task.
///
/// # Contoh Penggunaan
/// ```rust,no_run
/// use radar_emiten::SectorsClient;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let client = SectorsClient::from_env()?;
///     let reports = client.fetch_quarterly_reports("2024-07-01").await?;
///     println!("{} laporan ditemukan", reports.len());
///     Ok(())
/// }
/// ```
pub struct SectorsClient {
    /// HTTP client yang sudah dikonfigurasi.
    http: Client,

    /// API key dibaca dari environment — tidak disimpan sebagai `String` publik.
    api_key: Arc<String>,

    /// Credit tracker thread-safe.
    pub credits: Arc<CreditTracker>,

    /// Cache laporan kuartalan (key: since_date).
    quarterly_cache: Arc<MemCache<Vec<QuarterlyReport>>>,

    /// Cache screener per ticker (key: ticker).
    screener_cache: Arc<MemCache<ScreenerRow>>,

    /// Konfigurasi client.
    config: SectorsConfig,
}

impl SectorsClient {
    // -----------------------------------------------------------------------
    // Konstruktor
    // -----------------------------------------------------------------------

    /// Membuat client baru dengan API key yang sudah diketahui.
    ///
    /// Lebih aman menggunakan [`from_env`] agar key tidak masuk ke source code.
    pub fn new(api_key: String, config: SectorsConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .user_agent("radar-emiten/0.1 (IDX Sentinel Hackathon)")
            .build()
            .map_err(SectorsError::Http)?;

        Ok(Self {
            http,
            api_key: Arc::new(api_key),
            credits: CreditTracker::new(config.credit_budget),
            quarterly_cache: Arc::new(MemCache::with_24h_ttl()),
            screener_cache: Arc::new(MemCache::with_24h_ttl()),
            config,
        })
    }

    /// Membuat client dengan membaca `SECTORS_API_KEY` dari environment.
    ///
    /// Mendukung file `.env` melalui crate `dotenvy`.
    ///
    /// # Error
    /// Mengembalikan [`SectorsError::MissingApiKey`] jika variable tidak ada.
    pub fn from_env() -> Result<Self> {
        // Coba load .env jika ada (tidak error jika file tidak ditemukan)
        let _ = dotenvy::dotenv();

        let key = std::env::var(API_KEY_ENV).map_err(|_| SectorsError::MissingApiKey)?;

        if key.trim().is_empty() {
            return Err(SectorsError::MissingApiKey);
        }

        Self::new(key, SectorsConfig::default())
    }

    /// Membuat client dengan konfigurasi kustom dari environment.
    pub fn from_env_with_config(config: SectorsConfig) -> Result<Self> {
        let _ = dotenvy::dotenv();
        let key = std::env::var(API_KEY_ENV).map_err(|_| SectorsError::MissingApiKey)?;
        Self::new(key, config)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Membangun URL endpoint v2.
    fn url(&self, path: &str) -> String {
        format!("{}/{}", BASE_URL, path.trim_start_matches('/'))
    }

    /// Mengirim GET request ke Sectors API v2 dengan autentikasi.
    ///
    /// Secara otomatis menangani HTTP 410 (endpoint v1 gone).
    #[instrument(skip(self, params), fields(endpoint = %endpoint))]
    async fn get<T>(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = self.url(endpoint);
        debug!(url = %url, "Mengirim GET request");

        let response = self
            .http
            .get(&url)
            .header("Authorization", self.api_key.as_str())
            // Sectors API v2 juga mendukung header ini:
            .header("X-Api-Key", self.api_key.as_str())
            .query(params)
            .send()
            .await?;

        let status = response.status();

        match status {
            // HTTP 410 = endpoint v1 sudah dimatikan
            StatusCode::GONE => return Err(SectorsError::EndpointGone),

            // Sukses 2xx
            s if s.is_success() => {
                let text = response.text().await?;
                debug!(response_len = text.len(), "Menerima respons sukses");
                let parsed: T = serde_json::from_str(&text)?;
                Ok(parsed)
            }

            // Error lainnya
            _ => {
                let body = response.text().await.unwrap_or_default();
                warn!(status = status.as_u16(), body = %body, "API error");
                Err(SectorsError::ApiError {
                    status: status.as_u16(),
                    body,
                })
            }
        }
    }

    // -----------------------------------------------------------------------
    // Fungsi pengambilan data utama
    // -----------------------------------------------------------------------

    /// Mengambil daftar laporan kuartalan terbaru sejak tanggal tertentu.
    ///
    /// Endpoint: `GET /v2/companies/reports/quarterly?since=YYYY-MM-DD`
    ///
    /// # Arguments
    /// * `since_date` — Tanggal awal filter, format `"YYYY-MM-DD"`.
    ///
    /// # Credit Cost
    /// Dikenakan [`cost::QUARTERLY_REPORTS`] kredit per panggilan.
    ///
    /// # Cache
    /// Hasil dicache 24 jam dengan key = `since_date`.
    #[instrument(skip(self), fields(since = %since_date))]
    pub async fn fetch_quarterly_reports(
        &self,
        since_date: &str,
    ) -> Result<Vec<QuarterlyReport>> {
        // Cek cache terlebih dahulu
        if self.config.cache_enabled {
            if let Some(cached) = self.quarterly_cache.get(since_date) {
                info!(since = %since_date, "Quarterly reports dari cache");
                return Ok(cached);
            }
        }

        // Cek budget kredit
        if !self.credits.can_afford(cost::QUARTERLY_REPORTS) {
            warn!("Kredit tidak mencukupi untuk quarterly reports");
            return Err(SectorsError::Other(anyhow::anyhow!(
                "Kredit API habis. {}",
                self.credits.summary()
            )));
        }

        info!(since = %since_date, "Mengambil quarterly reports dari API");

        let response: QuarterlyReportResponse = self
            .get(
                "companies/reports/quarterly",
                &[("since", since_date)],
            )
            .await?;

        self.credits.charge(cost::QUARTERLY_REPORTS);
        info!(
            jumlah = response.data.len(),
            "{}",
            self.credits.summary()
        );

        // Simpan ke cache
        if self.config.cache_enabled {
            self.quarterly_cache.set(since_date, response.data.clone());
        }

        Ok(response.data)
    }

    /// Mengambil metrik keuangan kuartalan untuk satu ticker via Companies Screener v2.
    ///
    /// Endpoint: `GET /v2/companies/screener/?fields=...&where=symbol%3D%3DTICKER`
    ///
    /// # Arguments
    /// * `ticker` — Kode saham BEI (contoh: `"BBCA"`).
    /// * `fields` — Daftar field kuartalan yang diinginkan (contoh: `["net_interest_margin_q"]`).
    ///
    /// # Credit Cost
    /// Dikenakan [`cost::SCREENER_PER_TICKER`] kredit per ticker.
    ///
    /// # Cache
    /// Hasil per ticker dicache 24 jam.
    #[instrument(skip(self, fields), fields(ticker = %ticker))]
    pub async fn fetch_financial_metrics(
        &self,
        ticker: &str,
        fields: Vec<&str>,
    ) -> Result<ScreenerRow> {
        // Key cache mencakup ticker + fields agar field berbeda tidak saling override
        let cache_key = format!("{}:{}", ticker, fields.join(","));

        if self.config.cache_enabled {
            if let Some(cached) = self.screener_cache.get(&cache_key) {
                info!(ticker = %ticker, "Screener metrics dari cache");
                return Ok(cached);
            }
        }

        if !self.credits.can_afford(cost::SCREENER_PER_TICKER) {
            warn!(ticker = %ticker, "Kredit tidak mencukupi untuk screener");
            return Err(SectorsError::Other(anyhow::anyhow!(
                "Kredit API habis. {}",
                self.credits.summary()
            )));
        }

        info!(ticker = %ticker, ?fields, "Mengambil financial metrics dari screener");

        // Bangun field list — selalu sertakan symbol agar bisa diidentifikasi
        let mut all_fields = vec!["symbol", "company_name"];
        all_fields.extend_from_slice(&fields);
        let fields_str = all_fields.join(",");

        // Filter where clause: symbol==TICKER
        let where_clause = format!("symbol=={ticker}");

        let params: &[(&str, &str)] = &[
            ("fields", &fields_str),
            ("where", &where_clause),
        ];

        let response: ScreenerResponse = self.get("companies/screener/", params).await?;

        self.credits.charge(cost::SCREENER_PER_TICKER);

        // Ambil baris pertama (seharusnya hanya satu untuk filter ticker spesifik)
        let row = response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| SectorsError::FieldNotFound {
                ticker: ticker.to_string(),
                field: "all".to_string(),
            })?;

        if self.config.cache_enabled {
            self.screener_cache.set(&cache_key, row.clone());
        }

        Ok(row)
    }

    /// Mengambil metrics untuk banyak ticker sekaligus secara konkuren.
    ///
    /// Memanggil [`fetch_financial_metrics`] secara parallel untuk semua ticker.
    /// Ticker yang gagal akan menghasilkan error log tapi tidak menghentikan ticker lain.
    pub async fn fetch_metrics_batch(
        &self,
        tickers: Vec<&str>,
        fields: Vec<&str>,
    ) -> Vec<Result<ScreenerRow>> {
        let client = Arc::new(self as *const SectorsClient);
        let mut handles = Vec::new();

        for ticker in &tickers {
            let ticker = ticker.to_string();
            let fields: Vec<String> = fields.iter().map(|f| f.to_string()).collect();

            // Spawn concurrent tasks
            let http = self.http.clone();
            let api_key = Arc::clone(&self.api_key);
            let credits = Arc::clone(&self.credits);
            let screener_cache = Arc::clone(&self.screener_cache);
            let config = self.config.clone();

            handles.push(tokio::spawn({
                let ticker = ticker.clone();
                let fields_ref: Vec<&'static str> = Vec::new(); // placeholder
                async move {
                    // Buat client lightweight untuk task ini
                    let mini_client = SectorsClient {
                        http,
                        api_key,
                        credits,
                        quarterly_cache: Arc::new(MemCache::with_24h_ttl()),
                        screener_cache,
                        config,
                    };
                    let field_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
                    mini_client.fetch_financial_metrics(&ticker, field_refs).await
                }
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(join_err) => results.push(Err(SectorsError::Other(anyhow::anyhow!(
                    "Task join error: {join_err}"
                )))),
            }
        }
        results
    }

    // -----------------------------------------------------------------------
    // Konversi ke Finding
    // -----------------------------------------------------------------------

    /// Mengkonversi `ScreenerRow` menjadi `Vec<Finding>` untuk field yang diminta.
    ///
    /// Setiap field menghasilkan satu Finding jika ada data `_prev` (nilai sebelumnya).
    /// Konvensi penamaan field:
    /// - `"net_interest_margin_q"` → nilai sekarang
    /// - `"net_interest_margin_q_prev"` → nilai kuartal sebelumnya (jika tersedia di API)
    pub fn row_to_findings(
        &self,
        row: &ScreenerRow,
        fields: &[&str],
        periode: &str,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        for &field in fields {
            let nilai_sekarang = match row.get_f64(field) {
                Some(v) => v,
                None => {
                    debug!(
                        ticker = %row.symbol,
                        field = %field,
                        "Field tidak tersedia, skip"
                    );
                    continue;
                }
            };

            // Cari nilai sebelumnya: konvensi _prev suffix
            let prev_field = format!("{field}_prev");
            let nilai_sebelum = row.get_f64(&prev_field).unwrap_or(0.0);

            // Tentukan tingkat keyakinan berdasarkan ketersediaan data
            let tingkat_keyakinan = if row.get_f64(&prev_field).is_some() {
                TingkatKeyakinan::Tinggi
            } else {
                TingkatKeyakinan::Sedang // Data saat ini ada, tapi data prev tidak ada
            };

            findings.push(Finding::new(
                row.symbol.clone(),
                field,
                nilai_sebelum,
                nilai_sekarang,
                periode,
                "sectors_api_v2/screener",
                tingkat_keyakinan,
            ));
        }

        findings
    }

    // -----------------------------------------------------------------------
    // Utilitas
    // -----------------------------------------------------------------------

    /// Membersihkan entri cache yang sudah kadaluarsa.
    pub fn evict_cache(&self) {
        self.quarterly_cache.evict_expired();
        self.screener_cache.evict_expired();
    }

    /// Ringkasan status client.
    pub fn status(&self) -> String {
        format!(
            "SectorsClient | {} | quarterly_cache={} entries | screener_cache={} entries",
            self.credits.summary(),
            self.quarterly_cache.len(),
            self.screener_cache.len(),
        )
    }
}

// Implementasi Debug manual agar API key tidak muncul di log
impl std::fmt::Debug for SectorsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SectorsClient")
            .field("api_key", &"[REDACTED]")
            .field("credits_used", &self.credits.used())
            .field("quarterly_cache_len", &self.quarterly_cache.len())
            .field("screener_cache_len", &self.screener_cache.len())
            .finish()
    }
}
