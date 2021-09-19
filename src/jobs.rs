use tui::{
    backend::Backend,
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Block, Table, TableState},
    Terminal,
};

pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &reqwest::Client,
    key_rx: &mut tokio::sync::mpsc::Receiver<crate::events::Event>,
    project_name: &str,
    pipeline_id: &str,
) -> anyhow::Result<()> {
    let mut last_update = chrono::Local::now() - chrono::Duration::seconds(100);
    let mut jobs: Vec<crate::graphql::JobInfo> = Vec::new();
    let mut table_state = TableState::default();
    table_state.select(Some(0));

    loop {
        match key_rx.recv().await {
            None => return Ok(()),
            Some(event) => match event {
                crate::events::Event::Tick => (),
                crate::events::Event::Key(k) => match k {
                    termion::event::Key::Esc => return Ok(()),
                    termion::event::Key::Down => {
                        let mut cur_row = table_state.selected().unwrap_or(0);
                        cur_row += 1;
                        table_state.select(Some(cur_row));
                    }

                    termion::event::Key::PageDown => {
                        let mut cur_row = table_state.selected().unwrap_or(0);
                        cur_row += 10;
                        table_state.select(Some(cur_row));
                    }
                    termion::event::Key::Up => {
                        let mut cur_row = table_state.selected().unwrap_or(0);
                        if cur_row > 0 {
                            cur_row -= 1;
                            table_state.select(Some(cur_row));
                        }
                    }
                    termion::event::Key::PageUp => {
                        let mut cur_row = table_state.selected().unwrap_or(0);
                        if cur_row > 10 {
                            cur_row -= 10;
                            table_state.select(Some(cur_row));
                        }
                    }
                    termion::event::Key::Char('\n') => match table_state.selected() {
                        Some(row) if row < jobs.len() => {
                            crate::job_trace::run(terminal, client, key_rx, &jobs[row]).await?;
                        }
                        _ => (),
                    },
                    _ => (),
                },
            },
        }

        if (chrono::Local::now() - last_update) > chrono::Duration::seconds(30) {
            last_update = chrono::Local::now();

            jobs = crate::graphql::pipeline_jobs(client, project_name, pipeline_id).await?;
        }

        if let Some(row) = table_state.selected() {
            if row >= jobs.len() && !jobs.is_empty() {
                table_state.select(Some(jobs.len() - 1));
            }
        }

        terminal.draw(|f| {
            let rows = jobs.iter().map(|job| {
                tui::widgets::Row::new(vec![
                    tui::widgets::Cell::from(job.name.clone()),
                    (&job.status).into(),
                    tui::widgets::Cell::from(job.stage_name.clone()),
                    // tui::widgets::Cell::from(job.web_url.clone()),
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
            f.render_stateful_widget(table, f.size(), &mut table_state);
        })?;
    }
}

// #[derive(Debug, serde::Deserialize)]
// struct GitlabJob {
//     name: String,
//     id: u64,
//     web_url: String,
//     status: JobStatus,
// }

// #[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq, Hash)]
// #[serde(rename_all(deserialize = "lowercase"))]
// enum JobStatus {
//     Active,
//     Success,
//     Failed,
//     Running,
//     Skipped,
//     Manual,
//     Created,
//     Canceled,
// }

// impl std::fmt::Display for JobStatus {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Active => write!(f, "active"),
//             Self::Success => write!(f, "success"),
//             Self::Failed => write!(f, "failed"),
//             Self::Running => write!(f, "running"),
//             Self::Skipped => write!(f, "skipped"),
//             Self::Manual => write!(f, "manual"),
//             Self::Created => write!(f, "created"),
//             Self::Canceled => write!(f, "canceled"),
//         }
//     }
// }
