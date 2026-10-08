// src/sectors/client.rs — Sectors API v2 Client (inti)
//
// Endpoint mengikuti dokumentasi resmi (terverifikasi live 7 Okt 2026):
// - `GET /v2/subsectors/` — daftar sector/subsector slug (1 kredit).
// - `GET /v2/companies/` — Companies Screener, query `where`/`order_by`
//   (1 kredit per structured query; param `fields` TIDAK didukung API).
// - `GET /v2/companies/quarterly-financial-dates/` — feed tanggal laporan
//   kuartalan universe, param `since`/`limit` (1 kredit per halaman).
// - `GET /v2/financials/quarterly/{symbol}/` — keuangan kuartalan per emiten,
//   param `n_quarters` (1 kredit per quarter). Simbol menerima `BBCA` maupun
//   `BBCA.JK`; respons memakai suffix `.JK`.
// - `GET /v2/subsector/report/{sub}/` — laporan agregat subsektor,
//   param `sections` (1 kredit per section; default semua = 6 kredit).
//
// Referensi: https://docs.sectors.app/api-references/v2/indonesia/

use std::sync::Arc;

use reqwest::{Client, StatusCode};
use tracing::{debug, info, instrument, warn};

use crate::findings::{ConfidenceLevel, Finding};
use crate::sectors::{
    cache::MemCache,
    credits::{cost, CreditTracker},
    error::{Result, SectorsError},
    models::{
        quarterly::{QuarterlyDateRow, QuarterlyDatesResponse, QuarterlyFinancials},
        screener::{ScreenerResponse, ScreenerRow, SubsectorItem},
    },
};

/// Base URL Sectors API v2.
const BASE_URL: &str = "https://api.sectors.app/v2";

/// Environment variable yang menyimpan API key.
/// **JANGAN pernah hardcode API key di source code.**
const API_KEY_ENV: &str = "SECTORS_API_KEY";

/// Normalisasi simbol API menjadi ticker 4 huruf untuk kontrak internal.
///
/// API mengembalikan `"BBCA.JK"`; kontrak internal (orchestrator) validasi
/// `^[A-Z]{4}$`, sehingga suffix `.JK` harus dibuang:
/// `"BBCA.JK"` → `"BBCA"`, `"bbca"` → `"BBCA"`.
pub fn normalize_ticker(symbol: &str) -> String {
    let upper = symbol.trim().to_uppercase();
    upper
        .strip_suffix(".JK")
        .map(|s| s.to_string())
        .unwrap_or(upper)
}

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
            credit_budget: 1_000,
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
///     let dates = client.fetch_latest_quarterly_dates(Some("2026-01-01"), 5).await?;
///     println!("{} emiten terpantau", dates.len());
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

    /// Cache daftar subsektor (key tetap).
    subsectors_cache: Arc<MemCache<Vec<SubsectorItem>>>,

    /// Cache screener per filter (key: where-clause).
    screener_cache: Arc<MemCache<Vec<ScreenerRow>>>,

    /// Cache feed tanggal kuartalan (key: since+limit).
    quarterly_dates_cache: Arc<MemCache<Vec<QuarterlyDateRow>>>,

    /// Cache financials per ticker (key: ticker+n_quarters).
    financials_cache: Arc<MemCache<Vec<QuarterlyFinancials>>>,

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
            subsectors_cache: Arc::new(MemCache::with_24h_ttl()),
            screener_cache: Arc::new(MemCache::with_24h_ttl()),
            quarterly_dates_cache: Arc::new(MemCache::with_24h_ttl()),
            financials_cache: Arc::new(MemCache::with_24h_ttl()),
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
    /// Catatan billing resmi: respons 404 ikut menagih 1 kredit di sisi
    /// server; client hanya mencatat kredit untuk respons 2xx.
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

    /// Helper budget-check agar pesan error konsisten.
    fn check_budget(&self, amount: u64, label: &str) -> Result<()> {
        if self.credits.can_afford(amount) {
            return Ok(());
        }
        warn!("Kredit tidak mencukupi untuk {label}");
        Err(SectorsError::Other(anyhow::anyhow!(
            "Kredit API habis. {}",
            self.credits.summary()
        )))
    }

    // -----------------------------------------------------------------------
    // Fungsi pengambilan data utama
    // -----------------------------------------------------------------------

    /// Mengambil daftar sector/subsector slug resmi.
    ///
    /// Endpoint: `GET /v2/subsectors/` → JSON array `[{sector, subsector}]`.
    ///
    /// # Credit Cost
    /// Dikenakan [`cost::SUBSECTORS`] kredit per panggilan.
    ///
    /// # Cache
    /// Hasil dicache 24 jam (daftar ini jarang berubah).
    pub async fn fetch_subsectors(&self) -> Result<Vec<SubsectorItem>> {
        const KEY: &str = "all";

        if self.config.cache_enabled {
            if let Some(cached) = self.subsectors_cache.get(KEY) {
                info!("Subsectors dari cache");
                return Ok(cached);
            }
        }

        self.check_budget(cost::SUBSECTORS, "subsectors")?;
        info!("Mengambil subsectors dari API");

        let items: Vec<SubsectorItem> = self.get("subsectors/", &[]).await?;

        self.credits.charge(cost::SUBSECTORS);
        info!(jumlah = items.len(), "{}", self.credits.summary());

        if self.config.cache_enabled {
            self.subsectors_cache.set(KEY, items.clone());
        }

        Ok(items)
    }

    /// Menyaring emiten via Companies Screener.
    ///
    /// Endpoint: `GET /v2/companies/` dengan param `where` (SQL-like).
    /// Contoh filter subsektor: `"sub_sector = 'banks'"` (slug kebab-case dari
    /// [`fetch_subsectors`]).
    ///
    /// # Arguments
    /// * `subsector` — Slug subsektor (contoh: `"banks"`). `None` = tanpa filter.
    /// * `limit` — Jumlah hasil (API default 50).
    ///
    /// # Credit Cost
    /// Dikenakan [`cost::SCREENER_STRUCTURED`] kredit per panggilan.
    ///
    /// # Cache
    /// Hasil per filter dicache 24 jam.
    #[instrument(skip(self), fields(subsector = ?subsector))]
    pub async fn screen_companies(
        &self,
        subsector: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ScreenerRow>> {
        let where_clause;
        let cache_key = format!("{}:{limit}", subsector.unwrap_or("-"));

        if self.config.cache_enabled {
            if let Some(cached) = self.screener_cache.get(&cache_key) {
                info!("Screener dari cache");
                return Ok(cached);
            }
        }

        self.check_budget(cost::SCREENER_STRUCTURED, "screener")?;

        let limit_str = limit.to_string();
        let mut params: Vec<(&str, &str)> = vec![("limit", &limit_str)];
        if let Some(sub) = subsector {
            // Slug dikutip sesuai sintaks resmi: sub_sector = 'banks'
            where_clause = format!("sub_sector = '{sub}'");
            params.push(("where", &where_clause));
        }

        info!(?subsector, limit, "Menyaring emiten via screener");

        let response: ScreenerResponse = self.get("companies/", &params).await?;

        self.credits.charge(cost::SCREENER_STRUCTURED);
        info!(
            jumlah = response.results.len(),
            "{}",
            self.credits.summary()
        );

        if self.config.cache_enabled {
            self.screener_cache
                .set(&cache_key, response.results.clone());
        }

        Ok(response.results)
    }

    /// Mengambil feed tanggal laporan kuartalan terbaru untuk semua emiten.
    ///
    /// Endpoint: `GET /v2/companies/quarterly-financial-dates/` dengan param
    /// `since=YYYY-MM-DD` (hanya emiten yang lapor sejak tanggal itu) dan
    /// `limit` (maks 30 per halaman; feed penuh ~950 emiten).
    ///
    /// # Credit Cost
    /// Dikenakan [`cost::QUARTERLY_DATES_PAGE`] kredit per halaman.
    ///
    /// # Cache
    /// Hasil per kombinasi since+limit dicache 24 jam.
    #[instrument(skip(self), fields(since = ?since_date))]
    pub async fn fetch_latest_quarterly_dates(
        &self,
        since_date: Option<&str>,
        limit: u32,
    ) -> Result<Vec<QuarterlyDateRow>> {
        let limit = limit.clamp(1, 30);
        let cache_key = format!("{}:{limit}", since_date.unwrap_or("-"));

        if self.config.cache_enabled {
            if let Some(cached) = self.quarterly_dates_cache.get(&cache_key) {
                info!("Quarterly dates dari cache");
                return Ok(cached);
            }
        }

        self.check_budget(cost::QUARTERLY_DATES_PAGE, "quarterly dates")?;
        info!("Mengambil quarterly dates dari API");

        let limit_str = limit.to_string();
        let mut params: Vec<(&str, &str)> = vec![("limit", &limit_str)];
        if let Some(since) = since_date {
            params.push(("since", since));
        }

        let response: QuarterlyDatesResponse = self
            .get("companies/quarterly-financial-dates/", &params)
            .await?;

        self.credits.charge(cost::QUARTERLY_DATES_PAGE);
        info!(
            jumlah = response.results.len(),
            "{}",
            self.credits.summary()
        );

        if self.config.cache_enabled {
            self.quarterly_dates_cache
                .set(&cache_key, response.results.clone());
        }

        Ok(response.results)
    }

    /// Mengambil laporan agregat satu subsektor.
    ///
    /// Endpoint: `GET /v2/subsector/report/{sub}/` dengan param `sections`
    /// (comma-separated). Default API (tanpa `sections`) = 6 section = 6 kredit,
    /// sehingga method ini mewajibkan daftar section eksplisit.
    ///
    /// Section valid: `statistics`, `market_cap`, `stability`, `valuation`,
    /// `growth`, `companies`.
    ///
    /// Respons dikembalikan mentah (`serde_json::Value`) karena bentuknya
    /// mengikuti section yang diminta.
    ///
    /// # Credit Cost
    /// Dikenakan [`cost::SUBSECTOR_REPORT_PER_SECTION`] kredit per section.
    pub async fn fetch_subsector_report(
        &self,
        subsector: &str,
        sections: &[&str],
    ) -> Result<serde_json::Value> {
        if sections.is_empty() {
            return Err(SectorsError::Other(anyhow::anyhow!(
                "sections tidak boleh kosong (default API menagih 6 kredit)"
            )));
        }

        let cost = cost::SUBSECTOR_REPORT_PER_SECTION * sections.len() as u64;
        self.check_budget(cost, "subsector report")?;

        let sections_str = sections.join(",");
        let endpoint = format!("subsector/report/{subsector}/");

        info!(subsector = %subsector, sections = %sections_str, "Mengambil subsector report");

        let report: serde_json::Value = self
            .get(&endpoint, &[("sections", &sections_str)])
            .await?;

        self.credits.charge(cost);

        Ok(report)
    }

    /// Mengambil keuangan kuartalan N periode terakhir untuk satu ticker.
    ///
    /// Endpoint: `GET /v2/financials/quarterly/{ticker}/` dengan param
    /// `n_quarters`. Respons adalah JSON array (terbaru dulu).
    /// Ticker menerima `"BBCA"` maupun `"BBCA.JK"`.
    ///
    /// # Credit Cost
    /// Dikenakan [`cost::FINANCIALS_PER_QUARTER`] kredit per quarter
    /// yang dikembalikan.
    ///
    /// # Cache
    /// Hasil per ticker+n_quarters dicache 24 jam.
    #[instrument(skip(self), fields(ticker = %ticker))]
    pub async fn fetch_quarterly_financials(
        &self,
        ticker: &str,
        n_quarters: u32,
    ) -> Result<Vec<QuarterlyFinancials>> {
        let n_quarters = n_quarters.clamp(1, 12);
        let cache_key = format!("{}:{n_quarters}", ticker.to_uppercase());

        if self.config.cache_enabled {
            if let Some(cached) = self.financials_cache.get(&cache_key) {
                info!(ticker = %ticker, "Quarterly financials dari cache");
                return Ok(cached);
            }
        }

        // Budget dicek konservatif untuk n_quarters penuh.
        self.check_budget(
            cost::FINANCIALS_PER_QUARTER * n_quarters as u64,
            "quarterly financials",
        )?;

        let n_str = n_quarters.to_string();
        let endpoint = format!("financials/quarterly/{}/", ticker.trim().to_uppercase());

        info!(ticker = %ticker, n_quarters, "Mengambil quarterly financials");

        let records: Vec<QuarterlyFinancials> =
            self.get(&endpoint, &[("n_quarters", &n_str)]).await?;

        // Tagih sesuai jumlah quarter yang benar-benar dikembalikan.
        let billed = cost::FINANCIALS_PER_QUARTER * records.len().max(1) as u64;
        self.credits.charge(billed);

        if self.config.cache_enabled {
            self.financials_cache.set(&cache_key, records.clone());
        }

        Ok(records)
    }

    /// Mengambil financials untuk banyak ticker sekaligus (sekuensial).
    ///
    /// Biaya = jumlah ticker × [`cost::FINANCIALS_PER_QUARTER`] × `n_quarters`.
    /// Ticker yang gagal tidak menghentikan ticker lain (hasil `Err` per ticker).
    pub async fn fetch_financials_batch(
        &self,
        tickers: &[&str],
        n_quarters: u32,
    ) -> Vec<Result<Vec<QuarterlyFinancials>>> {
        let mut results = Vec::with_capacity(tickers.len());
        for ticker in tickers {
            results.push(self.fetch_quarterly_financials(ticker, n_quarters).await);
        }
        results
    }

    // -----------------------------------------------------------------------
    // Konversi ke Finding
    // -----------------------------------------------------------------------

    /// Mengkonversi 2 record kuartalan terbaru menjadi `Vec<Finding>`.
    ///
    /// Membandingkan record terbaru (`records[0]`) dengan periode pembanding
    /// (`records[1]`). Setiap field yang ada di kedua record menghasilkan satu
    /// `Finding`. Periode memakai tanggal laporan API (`YYYY-MM-DD`) apa adanya
    /// — tanpa ditebak menjadi label kuartal.
    ///
    /// Ticker dinormalisasi via [`normalize_ticker`] (`"BBCA.JK"` → `"BBCA"`)
    /// agar lolos validasi kontrak `^[A-Z]{4}$`.
    pub fn financials_to_findings(
        &self,
        records: &[QuarterlyFinancials],
        fields: &[&str],
        source: &str,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        let (current, previous) = match records {
            [c, p, ..] => (c, p),
            _ => {
                debug!("Butuh minimal 2 record kuartalan untuk perbandingan");
                return findings;
            }
        };

        let period = current.date.clone().unwrap_or_default();

        for &field in fields {
            let (Some(nilai_sekarang), Some(nilai_sebelum)) =
                (current.metric(field), previous.metric(field))
            else {
                debug!(field = %field, "Field tidak tersedia di kedua periode, skip");
                continue;
            };

            findings.push(Finding::new(
                current.ticker_short(),
                field,
                nilai_sebelum,
                nilai_sekarang,
                period.clone(),
                source,
                ConfidenceLevel::High,
            ));
        }

        findings
    }

    // -----------------------------------------------------------------------
    // Utilitas
    // -----------------------------------------------------------------------

    /// Membersihkan entri cache yang sudah kadaluarsa.
    pub fn evict_cache(&self) {
        self.subsectors_cache.evict_expired();
        self.screener_cache.evict_expired();
        self.quarterly_dates_cache.evict_expired();
        self.financials_cache.evict_expired();
    }

    /// Ringkasan status client.
    pub fn status(&self) -> String {
        format!(
            "SectorsClient | {} | subsectors={} screener={} dates={} financials={} entries",
            self.credits.summary(),
            self.subsectors_cache.len(),
            self.screener_cache.len(),
            self.quarterly_dates_cache.len(),
            self.financials_cache.len(),
        )
    }
}

// Implementasi Debug manual agar API key tidak muncul di log
impl std::fmt::Debug for SectorsClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SectorsClient")
            .field("api_key", &"[REDACTED]")
            .field("credits_used", &self.credits.used())
            .field("subsectors_cache_len", &self.subsectors_cache.len())
            .field("screener_cache_len", &self.screener_cache.len())
            .field(
                "quarterly_dates_cache_len",
                &self.quarterly_dates_cache.len(),
            )
            .field("financials_cache_len", &self.financials_cache.len())
            .finish()
    }
}
