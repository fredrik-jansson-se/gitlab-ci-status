use graphql_client::GraphQLQuery;

const GQL_URL: &str = "https://gitlab.com/api/graphql";
// type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/gitlab_schema.graphql",
    query_path = "graphql/project-pipelines.graphql",
    response_derives = "Debug",
    variable_derives = "Debug,Display,Clone"
)]
struct ProjectPipelines;

pub struct PipelineInfo {
    pub project_name: String,
    pub pipeline_iid: String,
    pub branch: String,
    pub web_url: String,
    pub status: project_pipelines::PipelineStatusEnum,
}
pub async fn project_pipelines(
    client: &reqwest::Client,
    name: &str,
) -> anyhow::Result<Vec<PipelineInfo>> {
    let variables = project_pipelines::Variables {
        name: name.to_string(),
    };

    let response_body =
        graphql_client::reqwest::post_graphql::<ProjectPipelines, _>(client, GQL_URL, variables)
            .await?;

    let project = response_body
        .data
        .and_then(|d| d.project)
        .ok_or(anyhow::Error::msg("Failed to get project data"))?;

    let pipelines = project
        .pipelines
        .ok_or(anyhow::Error::msg("Expected pipeline data for project"))?;

    let mut res = Vec::new();

    for pipeline in pipelines
        .nodes
        .ok_or(anyhow::Error::msg(format!("No pipelines for {}", name)))?
    {
        if let Some(pipeline) = pipeline {
            res.push(PipelineInfo {
                project_name: name.to_string(),
                pipeline_iid: pipeline.iid.to_string(),
                branch: pipeline
                    .ref_
                    .ok_or(anyhow::Error::msg("Failed to get branch name"))?,
                web_url: format!(
                    "https://www.gitlab.com{}",
                    pipeline
                        .path
                        .ok_or(anyhow::Error::msg("Failed to get url"))?
                ),
                status: pipeline.status,
            });
        }
    }

    Ok(res)
}

impl<'a> From<&project_pipelines::PipelineStatusEnum> for tui::widgets::Cell<'a> {
    fn from(ps: &project_pipelines::PipelineStatusEnum) -> tui::widgets::Cell<'a> {
        use tui::style::Style;
        use tui::widgets::Cell;
        let cell = Cell::from(format!("{:?}", ps));
        match ps {
            project_pipelines::PipelineStatusEnum::SUCCESS => {
                cell.style(Style::default().fg(tui::style::Color::Green))
            }
            project_pipelines::PipelineStatusEnum::FAILED => {
                cell.style(Style::default().fg(tui::style::Color::Red))
            }
            _ => cell,
        }
    }
}

type JobID = String;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/gitlab_schema.graphql",
    query_path = "graphql/pipeline-jobs.graphql",
    response_derives = "Debug",
    variable_derives = "Debug,Display,Clone"
)]
struct PipelineJobs;

#[derive(Debug)]
pub struct JobInfo {
    pub project_id: String,
    pub id: String,
    pub stage_name: String,
    pub name: String,
    pub status: pipeline_jobs::CiJobStatus,
}

pub async fn pipeline_jobs(
    client: &reqwest::Client,
    project_name: &str,
    pipeline_id: &str,
) -> anyhow::Result<Vec<JobInfo>> {
    let variables = pipeline_jobs::Variables {
        project_name: project_name.to_string(),
        pipeline_id: pipeline_id.to_string(),
    };

    let response_body =
        graphql_client::reqwest::post_graphql::<PipelineJobs, _>(client, GQL_URL, variables)
            .await?;

    // tracing::info!(?response_body);

    let project = response_body
        .data
        .and_then(|d| d.project)
        .ok_or(anyhow::Error::msg("Expected project data"))?;
    let pipeline = project
        .pipeline
        .ok_or(anyhow::Error::msg("Expected pipeline data for project"))?;

    // tracing::info!(?pipeline);

    let mut res = Vec::new();

    let stages = pipeline
        .stage_jobs
        .stages
        .and_then(|s| s.nodes)
        .ok_or(anyhow::Error::msg("No stages"))?;
    for stage in stages.into_iter() {
        if let Some(stage) = stage {
            let mut jobs = stage_jobs(project.id.clone(), stage)?;
            res.append(&mut jobs);
        }
    }

    let downstreams = pipeline.downstream.and_then(|d| d.nodes);
    if let Some(downstreams) = downstreams {
        // tracing::info!(?downstreams);
        for downstream in downstreams.into_iter() {
            if let Some(stages) = downstream.and_then(|ds| ds.stages) {
                for nodes in stages.nodes {
                    for stage_job in nodes {
                        if let Some(stage_job) = stage_job {
                            let mut jobs = stage_jobs(project.id.clone(), stage_job)?;
                            res.append(&mut jobs);
                        }
                    }
                }
            }
        }
    }

    res.sort_by(|j1, j2| match (&j1.status, &j2.status) {
        (pipeline_jobs::CiJobStatus::FAILED, _) => std::cmp::Ordering::Less,
        (_, pipeline_jobs::CiJobStatus::FAILED) => std::cmp::Ordering::Greater,
        (pipeline_jobs::CiJobStatus::RUNNING, _) => std::cmp::Ordering::Less,
        (_, pipeline_jobs::CiJobStatus::RUNNING) => std::cmp::Ordering::Greater,
        (_, _) => std::cmp::Ordering::Less,
    });

    Ok(res)
}

fn stage_jobs(
    project_id: String,
    stage_job: pipeline_jobs::StageJobsStagesNodes,
) -> anyhow::Result<Vec<JobInfo>> {
    let mut res = Vec::new();
    let stage_name = stage_job
        .name
        .ok_or(anyhow::Error::msg("Failed to get stage name"))?;
    let jobs = stage_job.jobs.and_then(|j| j.nodes);
    if let Some(jobs) = jobs {
        for job in jobs {
            if let Some(job) = job {
                res.push(JobInfo {
                    id: job.id.ok_or(anyhow::Error::msg("Failed to get job id"))?,
                    project_id: project_id.clone(),
                    stage_name: stage_name.clone(),
                    name: job
                        .name
                        .ok_or(anyhow::Error::msg("Failed to get job name"))?,
                    status: job
                        .status
                        .ok_or(anyhow::Error::msg("Failed to get job status"))?,
                });
            }
        }
    }
    // tracing::info!(?stage_job);
    Ok(res)
}
impl<'a> From<&pipeline_jobs::CiJobStatus> for tui::widgets::Cell<'a> {
    fn from(ps: &pipeline_jobs::CiJobStatus) -> tui::widgets::Cell<'a> {
        use tui::style::Style;
        let cell = tui::widgets::Cell::from(format!("{:?}", ps));
        match ps {
            pipeline_jobs::CiJobStatus::SUCCESS => {
                cell.style(Style::default().fg(tui::style::Color::Green))
            }
            pipeline_jobs::CiJobStatus::FAILED => {
                cell.style(Style::default().fg(tui::style::Color::Red))
            }
            _ => cell,
        }
    }
}
