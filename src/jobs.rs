use std::collections::HashMap;

//  printf '\e]8;;http://example.com\e\\This is a link\e]8;;\e\\\n'
//
use tui::{
    backend::Backend,
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Block, Table, TableState},
    Terminal,
};

const HELP_TEXT: &str = r#"
h               Close  help
ESC             Exit
up/down arrow   Select job
Enter           Trace job logs
R               Refresh jobs
"#;

pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &reqwest::Client,
    key_rx: &mut tokio::sync::mpsc::Receiver<crate::events::Event>,
    project_name: &str,
    pipeline_id: &str,
) -> anyhow::Result<()> {
    let mut last_update = chrono::Local::now();
    let mut jobs: Vec<crate::graphql::JobInfo> = Vec::new();
    let mut table_state = TableState::default();
    table_state.select(Some(0));
    let mut refresh = true;
    let mut help_height_percent = 0;

    let (jobs_updated_tx, mut jobs_updated_rx) =
        tokio::sync::watch::channel((chrono::Local::now(), Vec::new()));
    let jobs_updated_tx = std::sync::Arc::new(tokio::sync::Mutex::new(jobs_updated_tx));

    let mut jobs_per_type = HashMap::new();

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
                        let size = terminal.size()?;
                        cur_row += size.height as usize / 2;
                        cur_row = cur_row.min(jobs.len() - 1);
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
                        let size = terminal.size()?;
                        let half_height = size.height as usize / 2;
                        if cur_row > half_height {
                            cur_row -= half_height;
                        } else {
                            cur_row = 0;
                        }
                        table_state.select(Some(cur_row));
                    }
                    termion::event::Key::Char('\n') => match table_state.selected() {
                        Some(row) if row < jobs.len() => {
                            crate::job_trace::run(terminal, client, key_rx, &jobs[row]).await?;
                            // Reset colors from printing job trace
                            print!("{}", termion::style::Reset);
                        }
                        _ => (),
                    },
                    // termion::event::Key::Char('P') => match table_state.selected() {
                    //     Some(row) if row < jobs.len() => {
                    //         let project_id = project_name.replace("/", "%2F");
                    //         let job_id = jobs[row].job_id().unwrap();
                    //         let uri = format!(
                    //             "{}/projects/{}/jobs/{}/play",
                    //             crate::BASE_URL,
                    //             project_id,
                    //             job_id
                    //         );
                    //         let res = client.post(uri).send().await;
                    //         tracing::info!(?res);
                    //         if let Err(e) = res {
                    //             tracing::error!(
                    //                 "play job project_id: {} job_id: {}, error: {}",
                    //                 project_id,
                    //                 job_id,
                    //                 e
                    //             );
                    //         }
                    //     }
                    //     _ => (),
                    // },
                    termion::event::Key::Char('R') => {
                        refresh = true;
                    }
                    termion::event::Key::Char('h') => {
                        if help_height_percent > 0 {
                            help_height_percent = 0;
                        } else {
                            help_height_percent = 50;
                        }
                    }
                    _ => (),
                },
            },
        }

        if refresh || (chrono::Local::now() - last_update) > chrono::Duration::seconds(30) {
            // last_update = chrono::Local::now();
            refresh = false;

            // jobs = crate::graphql::pipeline_jobs(client, project_name, pipeline_id).await?;
            tokio::spawn(update_jobs(
                client.clone(),
                project_name.to_string(),
                pipeline_id.to_string(),
                jobs_updated_tx.clone(),
            ));
        }

        {
            let new_jobs = jobs_updated_rx.borrow_and_update();
            if new_jobs.0 != last_update {
                last_update = new_jobs.0;
                jobs = new_jobs.1.to_vec();
                jobs_per_type.clear();
                for job in jobs.iter() {
                    let entry = jobs_per_type.entry(job.status.clone()).or_insert(0);
                    *entry += 1;
                }
            }
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
                ])
            });

            let table = Table::new(rows)
                .block(Block::default().title(format!(
                        "Last updated: {}, {} jobs ({} pending) (h for help)",
                        last_update.format("%b %d %H:%M:%S"),
                        jobs.len(),
                        jobs_per_type
                            .get(&crate::graphql::CiJobStatus::PENDING)
                            .unwrap_or(&0)
                    )))
                .header(tui::widgets::Row::new(vec!["Name", "State", "Stage"]))
                .widths(&[
                    Constraint::Percentage(30),
                    Constraint::Percentage(20),
                    Constraint::Percentage(50),
                ])
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED),
                );

            let main_layout = tui::layout::Layout::default()
                .constraints(vec![
                    Constraint::Percentage(100 - help_height_percent),
                    Constraint::Percentage(help_height_percent),
                ])
                .direction(tui::layout::Direction::Vertical)
                .split(f.size());
            f.render_stateful_widget(table, main_layout[0], &mut table_state);

            let help = tui::widgets::Paragraph::new(HELP_TEXT);
            f.render_widget(help, main_layout[1]);
        })?;
    }
}

async fn update_jobs(
    client: reqwest::Client,
    project_name: String,
    pipeline_id: String,
    jobs_updated_tx: std::sync::Arc<
        tokio::sync::Mutex<
            tokio::sync::watch::Sender<(
                chrono::DateTime<chrono::Local>,
                Vec<crate::graphql::JobInfo>,
            )>,
        >,
    >,
) {
    let jobs = crate::graphql::pipeline_jobs(&client, &project_name, &pipeline_id).await;

    match jobs {
        Ok(jobs) => {
            let jobs_updated_tx = jobs_updated_tx.lock().await;
            let now = chrono::Local::now();
            let _ = jobs_updated_tx.send((now, jobs));
        }
        Err(e) => {
            tracing::error!("update_jobs: {} {} - {}", project_name, pipeline_id, e);
        }
    }
}
