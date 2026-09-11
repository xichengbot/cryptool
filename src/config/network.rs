use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    #[default]
    Auto,
    Direct,
    Http,
    Socks5,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxySettings {
    pub mode: ProxyMode,
    pub url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}
impl std::fmt::Debug for ProxySettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxySettings")
            .field("mode", &self.mode)
            .field("url_configured", &self.url.is_some())
            .field("credentials_configured", &self.username.is_some())
            .finish()
    }
}
impl ProxySettings {
    pub(crate) fn apply(
        &self,
        builder: reqwest::ClientBuilder,
    ) -> Result<reqwest::ClientBuilder, AppError> {
        let invalid = || {
            AppError::Config("invalid proxy configuration (check mode, URL and credentials)".into())
        };
        match self.mode {
            ProxyMode::Auto | ProxyMode::Direct => {
                if self.url.is_some() || self.username.is_some() || self.password.is_some() {
                    return Err(invalid());
                }
                Ok(if self.mode == ProxyMode::Direct {
                    builder.no_proxy()
                } else {
                    builder
                })
            }
            ProxyMode::Http | ProxyMode::Socks5 => {
                let url = reqwest::Url::parse(self.url.as_deref().ok_or_else(invalid)?)
                    .map_err(|_| invalid())?;
                let valid_scheme = match self.mode {
                    ProxyMode::Http => matches!(url.scheme(), "http" | "https"),
                    _ => matches!(url.scheme(), "socks5" | "socks5h"),
                };
                if !valid_scheme
                    || url.host_str().is_none()
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.query().is_some()
                    || url.fragment().is_some()
                    || !matches!(url.path(), "" | "/")
                    || self.username.is_some() != self.password.is_some()
                {
                    return Err(invalid());
                }
                let mut proxy = reqwest::Proxy::all(url).map_err(|_| invalid())?;
                if let (Some(user), Some(pass)) = (&self.username, &self.password) {
                    proxy = proxy.basic_auth(user, pass);
                }
                Ok(builder.no_proxy().proxy(proxy))
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct QuerySettings {
    pub max_concurrent: usize,
    pub request_timeout_ms: u64,
    pub total_timeout_ms: u64,
    pub max_upstream_requests: usize,
}
impl Default for QuerySettings {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            request_timeout_ms: 10_000,
            total_timeout_ms: 30_000,
            max_upstream_requests: 6,
        }
    }
}
impl QuerySettings {
    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if !(1..=64).contains(&self.max_concurrent)
            || !(1..=60_000).contains(&self.request_timeout_ms)
            || !(1..=120_000).contains(&self.total_timeout_ms)
            || !(1..=6).contains(&self.max_upstream_requests)
        {
            return Err(AppError::Config(
                "invalid exchange_data query budgets".into(),
            ));
        }
        Ok(())
    }
}
