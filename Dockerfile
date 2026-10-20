# PackForge API — multi-stage build producing a small, non-root image.
#   docker build -t packforge/api .
#   docker run -p 8080:8080 -v $PWD/keys.toml:/etc/packforge/keys.toml:ro packforge/api

FROM rust:1-slim-bookworm AS build
WORKDIR /src
# Manifests first so dependency builds are cached between source changes.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
COPY library library
COPY tests tests
RUN cargo build -p pf-api --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/pf-api /usr/local/bin/pf-api
ENV PF_BIND=0.0.0.0:8080 \
    PF_KEYS_FILE=/etc/packforge/keys.toml \
    RUST_LOG=info
EXPOSE 8080
USER nonroot
# Liveness/readiness: GET /v1/health (distroless has no shell for HEALTHCHECK).
ENTRYPOINT ["/usr/local/bin/pf-api"]
CMD ["serve"]
