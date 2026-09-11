use std::{net::SocketAddr, path::PathBuf};

use clap::Args;
use cryptool::{
    app::exchange_data::ExchangeDataService,
    server::{ServerOptions, router},
};

#[derive(Debug, Args)]
pub struct ServeCommand {
    /// Address on which the HTTP server listens.
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: SocketAddr,

    /// Directory containing a built web application's index.html and assets.
    #[arg(long)]
    web_dir: Option<PathBuf>,

    /// Additional accepted HTTP Host name. Repeat for multiple names.
    #[arg(long = "allow-host")]
    allow_hosts: Vec<String>,
}

impl ServeCommand {
    pub async fn execute(self, service: ExchangeDataService) -> anyhow::Result<()> {
        if !self.listen.ip().is_loopback() && self.allow_hosts.is_empty() {
            anyhow::bail!("non-loopback --listen requires at least one explicit --allow-host");
        }
        let mut options = ServerOptions::default();
        options.allowed_hosts.extend(self.allow_hosts);
        options.static_dir = self.web_dir;
        let app = router(service, options)?;
        let listener = tokio::net::TcpListener::bind(self.listen).await?;
        tracing::info!(listen = %self.listen, "cryptool HTTP server listening");
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok(())
    }
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to install Ctrl-C handler");
    }
}
