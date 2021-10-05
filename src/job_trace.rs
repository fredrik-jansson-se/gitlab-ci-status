use tui::{backend::Backend, Terminal};

const JUMP_HEIGHT_DIFF: isize = 3;

#[tracing::instrument(skip(terminal, client, key_rx))]
pub(crate) async fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    client: &reqwest::Client,
    key_rx: &mut tokio::sync::mpsc::Receiver<crate::events::Event>,
    job: &crate::graphql::JobInfo,
) -> anyhow::Result<()> {
    let project_id = job.project_id().unwrap();
    let job_id = job.job_id().unwrap();
    let uri = format!(
        "{}/projects/{}/jobs/{}/trace",
        crate::BASE_URL,
        project_id,
        job_id
    );

    let mut cur_row: isize = 0;
    let mut following = true;
    let mut last_update = chrono::Local::now() - chrono::Duration::seconds(100);
    let mut dirty = false;
    let mut logs: Vec<String> = Vec::new();

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
                    termion::event::Key::Up => {
                        dirty = true;
                        cur_row -= 1.min(cur_row);
                        following = false;
                    }
                    termion::event::Key::PageUp => {
                        dirty = true;
                        let height = terminal.size()?.height as isize - JUMP_HEIGHT_DIFF;
                        cur_row -= height.min(cur_row);
                        following = false;
                    }
                    termion::event::Key::Down => {
                        dirty = true;
                        cur_row += 1;
                        following = cur_row >= logs.len() as _;
                    }
                    termion::event::Key::PageDown => {
                        dirty = true;
                        let height = terminal.size()?.height as isize;
                        cur_row += height - JUMP_HEIGHT_DIFF;
                        following = cur_row >= logs.len() as _;
                    }
                    termion::event::Key::Char('g') => {
                        dirty = true;
                        following = false;
                        cur_row = 0;
                    }
                    termion::event::Key::Char('G') => {
                        dirty = true;
                        let height = terminal.size()?.height as isize;
                        if logs.len() > height as _ {
                            cur_row = logs.len() as isize - height;
                        } else {
                            cur_row = 0;
                        }
                    }
                    _ => (),
                },
            },
        }
        if (chrono::Local::now() - last_update) > chrono::Duration::seconds(10) {
            last_update = chrono::Local::now();
            let log_text = client.get(&uri).send().await?.text().await?;
            // let log_text = (0..100)
            //     .map(|v| v.to_string())
            //     .collect::<Vec<_>>()
            //     .join("\n");
            let width = terminal.size()?.width as usize - 1;
            logs = log_text.lines().flat_map(|s| cut_line(s, width)).collect();
            dirty = true;
        }

        if dirty {
            dirty = false;

            let height = terminal.size()?.height as usize;
            if following {
                let first_line = (logs.len() as i64 - height as i64).max(0) as usize;
                cur_row = first_line as _;
            }
            tracing::debug!(
                "cur_row: {}, logs: {}, height: {}",
                cur_row,
                logs.len(),
                height
            );
            terminal.clear()?;
            for log in logs.iter().skip(cur_row as _).take(height - 1) {
                print!("{}\r\n", log);
            }
        }
    }
}

// #[tracing::instrument(skip(terminal, client, key_rx))]
// pub(crate) async fn run_2<B: Backend>(
//     terminal: &mut Terminal<B>,
//     client: &reqwest::Client,
//     key_rx: &mut tokio::sync::mpsc::Receiver<crate::events::Event>,
//     job: &crate::graphql::JobInfo,
// ) -> anyhow::Result<()> {
//     let project_id = job
//         .project_id
//         .split("/")
//         .collect::<Vec<_>>()
//         .iter()
//         .rev()
//         .map(|s| s.to_string())
//         .next()
//         .unwrap();
//     let job_id = job
//         .id
//         .split("/")
//         .collect::<Vec<_>>()
//         .iter()
//         .rev()
//         .map(|s| s.to_string())
//         .next()
//         .unwrap();
//     let uri = format!(
//         "{}/projects/{}/jobs/{}/trace",
//         crate::BASE_URL,
//         project_id,
//         job_id
//     );

//     let mut cmd = tokio::process::Command::new("less")
//         .arg("+F")
//         .stdin(std::process::Stdio::piped())
//         // .stdout(std::process::Stdio::
//         .spawn()?;

//     let less_stdin = cmd
//         .stdin
//         .take()
//         .ok_or(anyhow::anyhow!("Failed to get stdin to less command"))?;

//     loop {
//         tokio::select! {
//             _ = cmd.wait() => {
//                 tracing::info!("less done");
//                 return Ok(());
//             }
//         }
//     }
// }

fn cut_line(text: &str, width: usize) -> Vec<String> {
    text.chars()
        .collect::<Vec<char>>()
        .chunks(width)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<String>>()
}

#[cfg(test)]
mod test {
    #[test]
    fn cut_lines() {
        let long_text = "abcdefghijklmnop";
        let cut_lines = super::cut_line(&long_text, 5);
        assert_eq!(cut_lines.as_ref(), vec!["abcde", "fghij", "klmno", "p"]);
    }
}
