use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub app_mode: AppMode,
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub llm_provider: String,
    pub deepseek_api_key: Option<String>,
    pub deepseek_model: Option<String>,
    pub gemini_api_key: Option<String>,
    pub gemini_model: Option<String>,
    pub whatsapp_access_token: Option<String>,
    pub whatsapp_phone_number_id: Option<String>,
    pub whatsapp_app_secret: Option<String>,
    pub whatsapp_verify_token: Option<String>,
    pub whatsapp_graph_api_version: Option<String>,
    pub whatsapp_template_name: Option<String>,
    pub whatsapp_template_language: Option<String>,
    pub admin_phone: Option<String>,
    pub internal_api_token: Option<String>,
    pub sectors_credit_budget: f64,
    pub max_alerts_per_day: u32,
    pub neutral_relative_threshold: f64,
    pub scheduler_interval_seconds: u64,
    pub mcp_transport: Option<String>,
    pub mcp_server_command: Option<String>,
    pub mcp_server_args: Option<String>,
    pub mcp_callback_base_url: Option<String>,
    pub policy_disclaimer_conflict_acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Mock,
    Real,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_mode: AppMode::Mock,
            host: "127.0.0.1".into(),
            port: 8080,
            database_url: "postgres://postgres:postgres@localhost:5434/idx_sentinel".into(),
            llm_provider: "deepseek".into(),
            deepseek_api_key: None,
            deepseek_model: None,
            gemini_api_key: None,
            gemini_model: None,
            whatsapp_access_token: None,
            whatsapp_phone_number_id: None,
            whatsapp_app_secret: None,
            whatsapp_verify_token: None,
            whatsapp_graph_api_version: None,
            whatsapp_template_name: None,
            whatsapp_template_language: None,
            admin_phone: None,
            internal_api_token: None,
            sectors_credit_budget: 1000.0,
            max_alerts_per_day: 3,
            neutral_relative_threshold: 0.02,
            scheduler_interval_seconds: 86400,
            mcp_transport: None,
            mcp_server_command: None,
            mcp_server_args: None,
            mcp_callback_base_url: None,
            policy_disclaimer_conflict_acknowledged: true,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let app_mode = match env::var("APP_MODE").as_deref() {
            Ok("real") => AppMode::Real,
            _ => AppMode::Mock,
        };

        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;

        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let deepseek_api_key = opt("DEEPSEEK_API_KEY");
        let deepseek_model = opt("DEEPSEEK_MODEL");
        let llm_provider = env::var("LLM_PROVIDER").unwrap_or_else(|_| {
            if deepseek_api_key.is_some() {
                "deepseek".to_string()
            } else {
                "gemini".to_string()
            }
        });

        let cfg = Self {
            app_mode,
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port,
            database_url,
            llm_provider,
            deepseek_api_key,
            deepseek_model,
            gemini_api_key: opt("GEMINI_API_KEY"),
            gemini_model: opt("GEMINI_MODEL"),
            whatsapp_access_token: opt("WHATSAPP_ACCESS_TOKEN"),
            whatsapp_phone_number_id: opt("WHATSAPP_PHONE_NUMBER_ID"),
            whatsapp_app_secret: opt("WHATSAPP_APP_SECRET"),
            whatsapp_verify_token: opt("WHATSAPP_VERIFY_TOKEN"),
            whatsapp_graph_api_version: opt("WHATSAPP_GRAPH_API_VERSION"),
            whatsapp_template_name: opt("WHATSAPP_TEMPLATE_NAME"),
            whatsapp_template_language: opt("WHATSAPP_TEMPLATE_LANGUAGE"),
            admin_phone: opt("ADMIN_PHONE"),
            internal_api_token: opt("INTERNAL_API_TOKEN"),
            sectors_credit_budget: env::var("SECTORS_CREDIT_BUDGET")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000.0),
            max_alerts_per_day: env::var("MAX_ALERTS_PER_DAY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            neutral_relative_threshold: env::var("NEUTRAL_RELATIVE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.02),
            scheduler_interval_seconds: env::var("SCHEDULER_INTERVAL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(86400),
            mcp_transport: opt("MCP_TRANSPORT"),
            mcp_server_command: opt("MCP_SERVER_COMMAND"),
            mcp_server_args: opt("MCP_SERVER_ARGS"),
            mcp_callback_base_url: opt("MCP_CALLBACK_BASE_URL"),
            policy_disclaimer_conflict_acknowledged: env::var(
                "POLICY_DISCLAIMER_CONFLICT_ACKNOWLEDGED",
            )
            .map(|v| v == "true")
            .unwrap_or(false),
        };

        if cfg.app_mode == AppMode::Real {
            cfg.validate_real_mode()?;
        }
        Ok(cfg)
    }

    fn validate_real_mode(&self) -> Result<(), ConfigError> {
        if self.deepseek_api_key.is_none() && self.gemini_api_key.is_none() {
            return Err(ConfigError::MissingRealCredential(
                "DEEPSEEK_API_KEY atau GEMINI_API_KEY",
            ));
        }
        let required = [
            ("WHATSAPP_ACCESS_TOKEN", &self.whatsapp_access_token),
            ("WHATSAPP_PHONE_NUMBER_ID", &self.whatsapp_phone_number_id),
            ("WHATSAPP_APP_SECRET", &self.whatsapp_app_secret),
            ("WHATSAPP_VERIFY_TOKEN", &self.whatsapp_verify_token),
            ("INTERNAL_API_TOKEN", &self.internal_api_token),
        ];
        for (name, value) in required {
            if value.is_none() {
                return Err(ConfigError::MissingRealCredential(name));
            }
        }
        Ok(())
    }
}

fn opt(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.is_empty())
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("variabel environment wajib belum diset: {0}")]
    Missing(&'static str),
    #[error("APP_MODE=real membutuhkan kredensial: {0}")]
    MissingRealCredential(&'static str),
}
