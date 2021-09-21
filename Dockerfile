# Build: docker build -t gitlab-status .
# Run: docker run --rm -it --env-file=.env gitlab-status

FROM rust:1.55 as build
WORKDIR /code
RUN apt-get update && apt-get install -y libssl-dev \
    && mkdir .cargo
COPY Cargo.toml Cargo.lock /code/
RUN cargo vendor > .cargo/config.toml
COPY src /code/src
COPY graphql /code/graphql
RUN cargo build --release

FROM debian:stable-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=build /code/target/release/gitlab-status /bin/
CMD ["gitlab-status"]
