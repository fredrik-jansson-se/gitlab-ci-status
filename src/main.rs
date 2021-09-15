use std::fmt;

use anyhow::Context;
use serde::Deserialize;

use termion::{raw::IntoRawMode, screen::AlternateScreen};
use tui::{backend::TermionBackend, Terminal};

mod events;
mod job_trace;
mod jobs;
mod pipelines;

const BASE_URL: &str = "https://www.gitlab.com/api/v4";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("logs.txt")?;
    let log_file = std::sync::Arc::new(log_file);
    tracing_subscriber::fmt::fmt().with_writer(log_file).init();

    let _ = dotenv::dotenv();
    let stdout = std::io::stdout().into_raw_mode()?;
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let access_key = std::env::var("GITLAB_ACCESS_TOKEN").context("GITLAB_ACCESS_TOKEN")?;
    let projects = std::env::var("PROJECT_IDS")
        .context("PROJECT_IDS")?
        .split(",")
        .map(|s| s.trim())
        .map(ToOwned::to_owned)
        .collect::<Vec<String>>();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "PRIVATE-TOKEN",
        reqwest::header::HeaderValue::from_str(&&access_key)?,
    );
    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()?;

    if let Err(e) = pipelines::run(&mut terminal, client, projects).await {
        tracing::error!(%e);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Project {
    name: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
enum PipelineStatus {
    created,
    waiting_for_resource,
    preparing,
    pending,
    running,
    success,
    failed,
    canceled,
    skipped,
    manual,
    scheduled,
}

impl fmt::Display for PipelineStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::created => write!(f, "created"),
            Self::waiting_for_resource => write!(f, "waiting_for_resource"),
            Self::preparing => write!(f, "preparing"),
            Self::pending => write!(f, "pending"),
            Self::running => write!(f, "running"),
            Self::success => write!(f, "success"),
            Self::failed => write!(f, "failed"),
            Self::canceled => write!(f, "canceled"),
            Self::skipped => write!(f, "skipped"),
            Self::manual => write!(f, "manual"),
            Self::scheduled => write!(f, "scheduled"),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Pipeline {
    id: u64,
    status: PipelineStatus,
    #[serde(rename = "ref")]
    reference: String,
    updated_at: chrono::DateTime<chrono::Local>,
    web_url: String,
    // username: Option<String>,
}

#[derive(Debug)]
struct Pipelines {
    project_id: String,
    project: String,
    pipelines: Vec<Pipeline>,
}
