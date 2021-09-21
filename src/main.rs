use anyhow::Context;

use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};

mod events;
mod graphql;
mod job_trace;
mod jobs;
mod pipelines;

const BASE_URL: &str = "https://www.gitlab.com/api/v4";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();
    if std::env::var("RUST_LOG").is_ok() {
        let log_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open("logs.txt")?;
        let log_file = std::sync::Arc::new(log_file);
        tracing_subscriber::fmt::fmt()
            .with_writer(log_file)
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    let access_key = std::env::var("GITLAB_ACCESS_TOKEN").context("GITLAB_ACCESS_TOKEN")?;
    let projects_names = std::env::var("PROJECT_NAMES")
        .context("PROJECT_NAMES")?
        .split(",")
        .map(|s| s.trim())
        .map(ToOwned::to_owned)
        .inspect(|name| tracing::info!(?name))
        .collect::<Vec<String>>();

    let mut headers = reqwest::header::HeaderMap::new();

    let mut private_token = reqwest::header::HeaderValue::from_str(&access_key)?;
    private_token.set_sensitive(true);
    headers.insert("PRIVATE-TOKEN", private_token);

    let mut auth_header =
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", access_key))?;
    auth_header.set_sensitive(true);
    headers.insert(reqwest::header::AUTHORIZATION, auth_header);

    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_str("application/json")?,
    );

    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        // .connection_verbose(true)
        .build()?;

    let stdout = std::io::stdout().into_raw_mode()?;
    let screen = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(screen);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;
    if let Err(e) = pipelines::run(&mut terminal, client, projects_names).await {
        tracing::error!(%e);
    }
    Ok(())
}
