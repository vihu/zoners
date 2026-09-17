# syntax=docker/dockerfile:1
FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --features web --bin zoners-web \
    && cp target/release/zoners-web /zoners-web

# One static binary: templates, assets, fonts and the tz database are compiled in.
FROM scratch
COPY --from=build /zoners-web /zoners-web
USER 65534:65534
ENTRYPOINT ["/zoners-web"]
