use std::fmt;

use anyhow::Context;
use serde::Deserialize;

const BASE_URL: &str = "https://www.gitlab.com/api/v4";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init_timed();

    let _ = dotenv::dotenv();

    let access_key = std::env::var("GITLAB_ACCESS_TOKEN").context("GITLAB_ACCESS_TOKEN")?;
    let projects = std::env::var("PROJECT_IDS")
        .context("PROJECT_IDS")?
        .split(",")
        .map(|s| s.trim())
        .map(ToOwned::to_owned)
        .collect::<Vec<String>>();

    let ref_match = std::env::var("MATCH_REF").unwrap_or(".*".to_string());
    let ref_match_re = regex::Regex::new(&ref_match)?;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "PRIVATE-TOKEN",
        reqwest::header::HeaderValue::from_str(&&access_key)?,
    );
    let client = reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()?;

    let (p_tx, mut p_rx) = tokio::sync::mpsc::channel(10);

    for pid in projects {
        let client = client.clone();
        let p_tx = p_tx.clone();
        let ref_match_re = ref_match_re.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = update_project(
                    client.clone(),
                    pid.clone(),
                    p_tx.clone(),
                    ref_match_re.clone(),
                )
                .await
                {
                    log::error!("Error {}", e);
                }
            }
        });
    }

    let mut pipelines = std::collections::HashMap::new();
    loop {
        let pipe = p_rx.recv().await;

        if let Some(pipe) = pipe {
            let results = pipelines.entry(pipe.project.clone()).or_insert(Vec::new());
            *results = pipe.pipelines;
        }
        let mut table = comfy_table::Table::new();
        table.set_header(vec!["Project", "Branch", "Last updated", "URL", "Status"]);
        for (name, results) in pipelines.iter() {
            for res in results {
                table.add_row(comfy_table::Row::from(vec![
                    name.into(),
                    (&res.reference).into(),
                    res.updated_at
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .into(),
                    (&res.web_url).into(),
                    res.status.to_cell(),
                ]));
            }
        }
        print!("\x1B[2J");
        println!("Last updated {}", chrono::Local::now());
        println!("{}", table);
    }
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

impl PipelineStatus {
    fn to_cell(&self) -> comfy_table::Cell {
        let text = self.to_string();
        match self {
            Self::success => comfy_table::Cell::new(text)
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Green),
            Self::failed => comfy_table::Cell::new(text)
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(comfy_table::Color::Red),
            _ => comfy_table::Cell::new(text),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Pipeline {
    id: u32,
    status: PipelineStatus,
    #[serde(rename = "ref")]
    reference: String,
    updated_at: chrono::DateTime<chrono::Local>,
    web_url: String,
    // username: Option<String>,
}

#[derive(Debug)]
struct Pipelines {
    project: String,
    pipelines: Vec<Pipeline>,
}

async fn update_project(
    client: reqwest::Client,
    project_id: String,
    p_tx: tokio::sync::mpsc::Sender<Pipelines>,
    ref_match_re: regex::Regex,
) -> anyhow::Result<()> {
    log::debug!("update project {}", project_id);
    let project_uri = format!("{}/projects/{}", BASE_URL, project_id);

    let project: Project = client.get(&project_uri).send().await?.json().await?;

    loop {
        log::info!("Updating project {}", project.name);
        let pipelines: Vec<Pipeline> = client
            .get(format!("{}/pipelines", project_uri))
            .send()
            .await?
            .json()
            .await?;

        let pipelines = pipelines
            .into_iter()
            .filter(|p| ref_match_re.is_match(&p.reference))
            .inspect(|pipe| log::info!("{:#?}", pipe))
            .take(5)
            .collect();

        for pipe in &pipelines {
            log::info!("{:#?}", pipe);
        }

        p_tx.send(Pipelines {
            project: project.name.clone(),
            pipelines,
        })
        .await?;

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
