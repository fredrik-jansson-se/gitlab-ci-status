use tui::{
    backend::Backend,
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Block, Cell, Table, TableState},
    Terminal,
};

const HELP_TEXT: &str = r#"
h               Close  help
ESC             Exit
up/down arrow   Select pipeline
Enter           List pipleline jobs
R               Refresh pipelines
"#;

pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: reqwest::Client,
    project_names: Vec<String>,
) -> anyhow::Result<()> {
    let (key_tx, mut key_rx) = tokio::sync::mpsc::channel(10);
    tokio::spawn(crate::events::event_handler(key_tx));

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    let mut refresh = true;
    let mut last_update = chrono::Local::now();
    let mut pipelines: Vec<crate::graphql::PipelineInfo> = Vec::new();
    let mut help_height_percent = 0;
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
                            if let Some(pipeline) = pipelines.get(row) {
                                crate::jobs::run(
                                    terminal,
                                    &client,
                                    &mut key_rx,
                                    &pipeline.project_name,
                                    &pipeline.pipeline_iid,
                                )
                                .await?;
                            }
                        }
                    }
                    termion::event::Key::Char('h') => {
                        if help_height_percent != 0 {
                            help_height_percent = 0;
                        } else {
                            help_height_percent = 50;
                        }
                    }
                    termion::event::Key::Char('R') => {
                        refresh = true;
                    }
                    _k => {
                        // tracing::error!(?k);
                        ()
                    }
                },
            },
        }

        if refresh || (chrono::Local::now() - last_update) > chrono::Duration::seconds(30) {
            refresh = false;
            last_update = chrono::Local::now();
            pipelines.clear();
            for project_name in project_names.iter() {
                let mut new_pipelines =
                    crate::graphql::project_pipelines(&client, &project_name).await?;
                pipelines.append(&mut new_pipelines);
            }
        }

        terminal.draw(|f| {
            let mut rows = Vec::new();
            for pipeline in pipelines.iter() {
                rows.push(pipeline_to_row(pipeline));
            }

            if let Some(row) = table_state.selected() {
                if row >= rows.len() && !rows.is_empty() {
                    table_state.select(Some(rows.len() - 1));
                }
            }
            let table = Table::new(rows)
                .block(Block::default().title(format!(
                    "Last updated: {} (h for help)",
                    last_update.format("%b %d %H:%M:%S")
                )))
                .header(tui::widgets::Row::new(vec![
                    "Project",
                    "Branch",
                    "Created At",
                    "URL",
                    "Status",
                ]))
                .widths(&[
                    Constraint::Percentage(15),
                    Constraint::Percentage(20),
                    Constraint::Percentage(15),
                    Constraint::Percentage(40),
                    Constraint::Percentage(10),
                ])
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

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

fn pipeline_to_row<'a>(pipeline: &crate::graphql::PipelineInfo) -> tui::widgets::Row<'a> {
    let project_name = Cell::from(pipeline.project_name.clone());
    tui::widgets::Row::new(vec![
        project_name,
        pipeline.branch.clone().into(),
        pipeline
            .created_at
            .format("%b %d %H:%M:%S")
            .to_string()
            .into(),
        pipeline.web_url.clone().into(),
        (&pipeline.status).into(),
    ])
}
