# Monitor CI Pipelines
Create a config.yaml file.

The gitlab-access-token is obtained from gitlab and needs **api_read** rights.

**NOTE** First 2 * num-pipelines pipelines are fetched from Gitlab, then the match-branch-re regular expression is used as a filter. Therefore there may be less than num-pipelines pipelines rendered.

## Example config.yaml
```yaml
gitlab-access-token: xxx
projects:
  - name: avassa/code
    # Default is 5
    # num-pipelines: 6

    # Match branch name against this regex
    # match-branch-re: "master"

  - name: avassa/control-tower
  - name: avassa/system-e2e-test
```

## Running
## Docker
```shell
docker run --rm -it -vconfig.yaml:/config.yaml ghcr.io/fredrik-jansson-se/gitlab-ci-status:master
```
### From source
```shell
cargo run --release
```
