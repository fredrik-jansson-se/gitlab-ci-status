use futures_util::StreamExt;
use tui::{
    backend::Backend,
    layout::Constraint,
    style::{Modifier, Style},
    text::{Span, Spans},
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
    let uri = format!(
        "{}/projects/{}/jobs/{}/trace",
        crate::BASE_URL,
        project_id,
        job_id
    );

    let mut logs = Vec::new();
    let mut logstream = client.get(uri).send().await?.bytes_stream();
    let mut log_strings = Vec::new();
    terminal.clear()?;
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
                    _ => (),
                },
            },
        }
        if let Some(Ok(buf)) = logstream.next().await {
            logs.push(buf);
            let log_string = String::from_utf8(logs.iter().flatten().cloned().collect())?;
            // tracing::info!("No lines: {}", log_spans.len());
            let mut new_log_strings = log_string.lines().map(|s| s.to_string()).collect();
            terminal.clear()?;
            for l in &new_log_strings {
                print!("{}\r\n", l);
            }
            log_strings.append(&mut new_log_strings);
        }
    }
}
