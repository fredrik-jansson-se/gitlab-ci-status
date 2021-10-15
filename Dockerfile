# Build: docker build -t gitlab-status .
# Run: docker run --rm -it --env-file=.env gitlab-status

FROM rust:alpine as build
WORKDIR /code
ENV RUSTFLAGS="-Ctarget-feature=-crt-static"
RUN apk add libc-dev openssl-dev \
    && mkdir .cargo
COPY Cargo.toml Cargo.lock /code/
RUN cargo vendor > .cargo/config.toml
COPY src /code/src
COPY graphql /code/graphql
RUN cargo build --release 
RUN strip /code/target/release/gitlab-status

FROM alpine
RUN apk add --no-cache libgcc
COPY --from=build /code/target/release/gitlab-status /bin/
CMD ["gitlab-status", "-c", "/config.yaml"]
