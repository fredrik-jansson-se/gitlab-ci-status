use tui::{
    backend::Backend,
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Block, Cell, Table, TableState},
    Terminal,
};

pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: reqwest::Client,
    projects: Vec<String>,
) -> anyhow::Result<()> {
    let (key_tx, mut key_rx) = tokio::sync::mpsc::channel(10);
    tokio::spawn(crate::events::event_handler(key_tx));

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    let mut last_update = chrono::Local::now() - chrono::Duration::seconds(100);
    let mut pipelines: Vec<(String, String, crate::Pipeline)> = Vec::new();
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
                    termion::event::Key::Up => {
                        let mut cur_row = table_state.selected().unwrap_or(0);
                        if cur_row > 0 {
                            cur_row -= 1;
                            table_state.select(Some(cur_row));
                        }
                    }
                    termion::event::Key::Char('\n') => {
                        if let Some(row) = table_state.selected() {
                            if let Some((project_id, _, pipeline)) = pipelines.get(row) {
                                crate::jobs::run(
                                    terminal,
                                    &client,
                                    &mut key_rx,
                                    project_id,
                                    pipeline.id,
                                )
                                .await?;
                            }
                        }
                    }
                    _k => {
                        // tracing::error!(?k);
                        ()
                    }
                },
            },
        }

        if (chrono::Local::now() - last_update) > chrono::Duration::seconds(30) {
            last_update = chrono::Local::now();
            let updated_pipes = get_pipelines(&client, &projects).await?;
            pipelines.clear();
            for project in updated_pipes.into_iter() {
                for pipeline in project.pipelines.into_iter() {
                    pipelines.push((
                        project.project_id.clone(),
                        project.project.clone(),
                        pipeline,
                    ));
                }
            }
        }

        terminal.draw(|f| {
            let mut rows = Vec::new();
            for (_, project, pipeline) in pipelines.iter() {
                rows.push(pipeline_to_row(project, pipeline));
            }

            if let Some(row) = table_state.selected() {
                if row >= rows.len() && !rows.is_empty() {
                    table_state.select(Some(rows.len() - 1));
                }
            }
            let table = Table::new(rows)
                .block(Block::default().title(format!(
                    "Last updated: {}",
                    last_update.format("%Y-%m-%d %H:%M:%S")
                )))
                .header(tui::widgets::Row::new(vec![
                    "Project", "Branch", "URL", "Status",
                ]))
                .widths(&[
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                    Constraint::Percentage(50),
                    Constraint::Min(10),
                ])
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            f.render_stateful_widget(table, f.size(), &mut table_state);
        })?;
    }
}

fn pipeline_to_row<'a>(project_name: &'a str, pipeline: &crate::Pipeline) -> tui::widgets::Row<'a> {
    let project_name = Cell::from(project_name);
    tui::widgets::Row::new(vec![
        project_name,
        pipeline.reference.clone().into(),
        pipeline.web_url.clone().into(),
        (&pipeline.status).into(),
    ])
}

impl<'a> From<&crate::PipelineStatus> for Cell<'a> {
    fn from(ps: &crate::PipelineStatus) -> Cell<'a> {
        let cell = Cell::from(ps.to_string());
        match ps {
            crate::PipelineStatus::success => {
                cell.style(Style::default().fg(tui::style::Color::Green))
            }
            crate::PipelineStatus::failed => {
                cell.style(Style::default().fg(tui::style::Color::Red))
            }
            _ => cell,
        }
    }
}

async fn get_pipelines(
    client: &reqwest::Client,
    projects: &[String],
) -> anyhow::Result<Vec<crate::Pipelines>> {
    let mut res = Vec::new();

    let mut jh = Vec::new();

    for project_id in projects {
        jh.push(tokio::spawn(get_project_pipelines(
            client.clone(),
            project_id.to_string(),
        )));
    }

    for j in jh.iter_mut() {
        let pp = j.await??;
        res.push(pp);
    }

    Ok(res)
}

async fn get_project_pipelines(
    client: reqwest::Client,
    project_id: String,
) -> anyhow::Result<crate::Pipelines> {
    let ref_match = std::env::var("MATCH_REF").unwrap_or(".*".to_string());
    let ref_match_re = regex::Regex::new(&ref_match)?;
    let project_uri = format!("{}/projects/{}", crate::BASE_URL, project_id);

    let project: crate::Project = client.get(&project_uri).send().await?.json().await?;

    log::info!("Updating project {}", project.name);
    let pipelines: Vec<crate::Pipeline> = client
        .get(format!("{}/pipelines", project_uri))
        .send()
        .await?
        .json()
        .await?;

    let pipelines = pipelines
        .into_iter()
        .filter(|p| ref_match_re.is_match(&p.reference))
        .inspect(|pipe| log::debug!("{:#?}", pipe))
        .take(5)
        .collect();

    Ok(crate::Pipelines {
        project_id,
        project: project.name,
        pipelines,
    })
}
