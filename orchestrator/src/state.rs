use sqlx::PgPool;

use crate::config::Config;
use crate::services::whatsapp_client::WhatsAppClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub http: reqwest::Client,
    pub whatsapp: WhatsAppClient,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let whatsapp = WhatsAppClient::new(&config);
        Self {
            pool,
            config,
            http,
            whatsapp,
        }
    }
}
