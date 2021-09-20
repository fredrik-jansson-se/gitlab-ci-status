use tui::{backend::Backend, Terminal};

#[tracing::instrument(skip(terminal, client, key_rx))]
pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &reqwest::Client,
    key_rx: &mut tokio::sync::mpsc::Receiver<crate::events::Event>,
    job: &crate::graphql::JobInfo,
) -> anyhow::Result<()> {
    tracing::info!("run");
    let project_id = job
        .project_id
        .split("/")
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .map(|s| s.to_string())
        .next()
        .unwrap();
    let job_id = job
        .id
        .split("/")
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .map(|s| s.to_string())
        .next()
        .unwrap();
    let uri = format!(
        "{}/projects/{}/jobs/{}/trace",
        crate::BASE_URL,
        project_id,
        job_id
    );

    // let get_jobs = async { client.get(uri).send().await?.text().await };

    let mut cur_row = 0;
    let mut following = true;
    let mut last_update = chrono::Local::now() - chrono::Duration::seconds(100);
    let mut dirty = false;
    let mut logs = Vec::new();

    loop {
        match key_rx.recv().await {
            None => return Ok(()),
            Some(event) => match event {
                crate::events::Event::Tick => (),
                crate::events::Event::Key(k) => match k {
                    termion::event::Key::Esc => {
                        terminal.clear()?;
                        return Ok(());
                    }
                    termion::event::Key::PageUp => {
                        dirty = true;
                        let height = terminal.size()?.height as usize;
                        cur_row -= height.min(cur_row);
                        following = false;
                    }
                    termion::event::Key::PageDown => {
                        dirty = true;
                        let height = terminal.size()?.height as usize;
                        cur_row += height;
                        following = cur_row >= logs.len();
                    }
                    _ => (),
                },
            },
        }
        if (chrono::Local::now() - last_update) > chrono::Duration::seconds(10) {
            last_update = chrono::Local::now();
            let log_text = client.get(&uri).send().await?.text().await?;
            logs = log_text.lines().map(|s| s.to_string()).collect();
            dirty = true;
        }

        if dirty {
            dirty = false;

            let height = terminal.size()?.height as usize;
            if following {
                let first_line = (logs.len() as i64 - height as i64).max(0) as usize;
                cur_row = first_line;
            }
            tracing::info!(
                "cur_row: {}, logs: {}, height: {}",
                cur_row,
                logs.len(),
                height
            );
            terminal.clear()?;
            for log in logs.iter().skip(cur_row).take(height) {
                print!("{}\r\n", log);
            }
        }
    }
}
