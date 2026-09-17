# syntax=docker/dockerfile:1
FROM rust:1-alpine AS build

RUN apk add --no-cache musl-dev
WORKDIR /build

COPY Cargo.toml Cargo.lock README.md ./
COPY src/ src/
COPY templates/*.html templates/
COPY assets/ assets/
# The cache mounts only speed up local rebuilds. The binary is copied out because the target mount is not part of the layer.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked --features web --bin zoners-web \
    && cp target/release/zoners-web /zoners-web

# One static binary: templates, assets, fonts and the tz database are compiled in.
FROM scratch

COPY --from=build /zoners-web /zoners-web
COPY LICENSE /usr/share/licenses/zoners/LICENSE
COPY assets/fonts/OFL.txt /usr/share/licenses/zoners/OFL.txt

LABEL org.opencontainers.image.source="https://github.com/vihu/zoners"
LABEL org.opencontainers.image.licenses="AGPL-3.0-only"
USER 65534:65534
EXPOSE 5051
ENTRYPOINT ["/zoners-web"]
