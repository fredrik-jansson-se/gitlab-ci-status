// https://github.com/linkerd/linkerd-await/blob/57590fc9c808216a879f56be2c181d5353b397cc/src/main.rs

use clap::{crate_authors, crate_description, crate_name, crate_version};
use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};

mod config;
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

    let matches = clap::app_from_crate!()
        .arg(
            clap::Arg::with_name("config-file")
                .short("c")
                .long("config-file")
                .takes_value(true)
                .default_value("config.yaml")
                .help("Path to config.yaml"),
        )
        .get_matches();

    let cfg_file = matches.value_of("config-file").unwrap();

    let cfg = config::load_config(&cfg_file)?;

    let mut headers = reqwest::header::HeaderMap::new();

    let mut private_token = reqwest::header::HeaderValue::from_str(&cfg.gitlab_access_token)?;
    private_token.set_sensitive(true);
    headers.insert("PRIVATE-TOKEN", private_token);

    let mut auth_header =
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", cfg.gitlab_access_token))?;
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
    if let Err(e) = pipelines::run(&mut terminal, client, cfg.projects).await {
        tracing::error!(%e);
    }
    Ok(())
}
