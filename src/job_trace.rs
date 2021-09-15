use tui::{
    backend::Backend,
    layout::Constraint,
    style::{Modifier, Style},
    widgets::{Block, Cell, Paragraph, Table, TableState},
    Terminal,
};

#[tracing::instrument(skip(terminal, client, key_rx))]
pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &reqwest::Client,
    key_rx: &mut tokio::sync::mpsc::Receiver<crate::events::Event>,
    project_id: &str,
    job_id: u64,
) -> anyhow::Result<()> {
    tracing::info!("run");
    let mut last_update = chrono::Local::now() - chrono::Duration::seconds(100);
    let mut logs = String::new();
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
                "{}/projects/{}/jobs/{}/trace",
                crate::BASE_URL,
                project_id,
                job_id
            );

            logs = client.get(uri).send().await?.text().await?;
            // tracing::error!(?jobs);
        }

        terminal.draw(|f| {
            let size = f.size();
            // tracing::info!(?size);
            let log_lines = logs.lines().count() as u16;
            let scroll_offset = if log_lines > size.height {
                log_lines - size.height
                // 10
            } else {
                0
            };
            let paragraph = Paragraph::new(logs.clone()).scroll((scroll_offset, 0));
            f.render_widget(paragraph, f.size());
        });
        //GET /projects/:id/jobs/:job_id/trace
    }
    // tracing::info!(?jobs_json);
    Ok(())
}
