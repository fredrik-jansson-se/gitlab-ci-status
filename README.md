# Monitor CI Pipelines
Set the following environment variables, or put them in a `.env` file.

The GITLAB_ACCESS_TOKEN is obtained from gitlab and needs **api_read** rights.

## Example .env
```shell
# Gitlab access token.
GITLAB_ACCESS_TOKEN=<my access key>
# Comma separated list of project names
PROJECT_NAMES=foo-group/project1,foo-group/project2
# Match branch names against this regular expression (optional)
MATCH_REF=master|(.*merge-requests.*)
```

## Running
## Docker
```shell
docker run --rm -it --env-file=.env ghcr.io/fredrik-jansson-se/gitlab-ci-status:master
```
### From source
```shell
cargo run --release
```
