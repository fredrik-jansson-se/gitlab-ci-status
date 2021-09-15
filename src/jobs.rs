use tui::{
    backend::Backend,
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Block, Cell, Table, TableState},
    Terminal,
};

pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &reqwest::Client,
    key_rx: &mut tokio::sync::mpsc::Receiver<crate::events::Event>,
    project_id: &str,
    pipeline_id: u64,
) -> anyhow::Result<()> {
    let mut last_update = chrono::Local::now() - chrono::Duration::seconds(100);
    let mut jobs: Vec<GitlabJob> = Vec::new();
    loop {
        match key_rx.recv().await {
            None => return Ok(()),
            Some(event) => match event {
                crate::events::Event::Tick => (),
                crate::events::Event::Key(k) => match k {
                    termion::event::Key::Esc => return Ok(()),
                    _ => (),
                },
            },
        }

        if (chrono::Local::now() - last_update) > chrono::Duration::seconds(30) {
            last_update = chrono::Local::now();
            let uri = format!(
                "{}/projects/{}/pipelines/{}/jobs",
                crate::BASE_URL,
                project_id,
                pipeline_id
            );
            let jobs_json: serde_json::Value = client.get(uri).send().await?.json().await?;
            let f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open("jobs.json")?;

            let _ = serde_json::to_writer_pretty(f, &jobs_json);
            jobs = serde_json::from_value(jobs_json)?;
            jobs.sort_by(|j1, j2| j1.id.cmp(&j2.id));
            // tracing::error!(?jobs);
        }

        terminal.draw(|f| {
            let rows = jobs.iter().map(|job| {
                tui::widgets::Row::new(vec![
                    tui::widgets::Cell::from(job.name.clone()),
                    (&job.status).into(),
                    tui::widgets::Cell::from(job.web_url.clone()),
                ])
            });

            let table = Table::new(rows)
                .block(Block::default().title(format!(
                    "Last updated: {}",
                    last_update.format("%Y-%m-%d %H:%M:%S")
                )))
                .header(tui::widgets::Row::new(vec!["Name", "State", "URL"]))
                .widths(&[
                    Constraint::Percentage(30),
                    Constraint::Percentage(20),
                    Constraint::Percentage(50),
                ])
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(table, f.size());
        });
    }
}

#[derive(Debug, serde::Deserialize)]
struct GitlabJob {
    name: String,
    id: u64,
    web_url: String,
    status: JobStatus,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all(deserialize = "lowercase"))]
enum JobStatus {
    Active,
    Success,
    Failed,
    Running,
    Skipped,
    Manual,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
            Self::Running => write!(f, "running"),
            Self::Skipped => write!(f, "skipped"),
            Self::Manual => write!(f, "manual"),
        }
    }
}

impl<'a> From<&JobStatus> for Cell<'a> {
    fn from(ps: &JobStatus) -> Cell<'a> {
        let cell = Cell::from(ps.to_string());
        match ps {
            JobStatus::Success => cell.style(Style::default().fg(tui::style::Color::Green)),
            JobStatus::Failed => cell.style(Style::default().fg(tui::style::Color::Red)),
            _ => cell,
        }
    }
}
