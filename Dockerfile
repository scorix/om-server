FROM rust:bookworm AS builder

ENV CARGO_NET_RETRY=10
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
ENV CARGO_HTTP_MULTIPLEXING=false

RUN apt-get update \
    && apt-get install -y --no-install-recommends clang libclang-dev git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependency downloads separately from source changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && printf 'pub fn _docker_dep_cache() {}\n' > src/lib.rs \
    && printf 'fn main() {}\n' > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    for attempt in 1 2 3 4 5; do \
      cargo fetch --locked && break; \
      echo "cargo fetch failed (attempt ${attempt}), retrying..." >&2; \
      sleep $((attempt * 5)); \
    done

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin om-server \
    && cp /app/target/release/om-server /tmp/om-server

FROM debian:bookworm-slim

ARG GRPCURL_VERSION=1.9.3

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && ARCH="$(dpkg --print-architecture)" \
    && case "$ARCH" in \
         amd64) GRPCURL_ARCH=x86_64 ;; \
         arm64) GRPCURL_ARCH=arm64 ;; \
         *) echo "unsupported architecture: $ARCH" >&2; exit 1 ;; \
       esac \
    && curl -fsSL \
         "https://github.com/fullstorydev/grpcurl/releases/download/v${GRPCURL_VERSION}/grpcurl_${GRPCURL_VERSION}_linux_${GRPCURL_ARCH}.tar.gz" \
       | tar -xz -C /usr/local/bin grpcurl \
    && apt-get purge -y curl \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /tmp/om-server /usr/local/bin/om-server

EXPOSE 50051
ENTRYPOINT ["om-server"]
