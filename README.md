# Monitor CI Pipelines
Set the following environment variables, or put them in a `.env` file.

The GITLAB_ACCESS_TOKEN is obtained from gitlab and needs **api_read** rights.

## Example .env
```shell
# Gitlab access token.
GITLAB_ACCESS_TOKEN=<my access key>
# Comma separated list of project ids
PROJECT_IDS=1,2,3
# Match ref against this regular expression
MATCH_REF="master|(.*merge-requests.*)"
```

## Running
cargo run --release
