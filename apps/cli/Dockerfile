# tok0 container image — multi-stage, ~10MB final.
#
# Build:   docker build -t tok0:local .
# Run:     docker run --rm -it -v "$PWD":/work -w /work tok0:local git status
#
# The image uses the musl target so the final stage is a scratch-like
# distroless image with no libc dependencies.

# ────────────────────────────────────────────────────────────────────
# Stage 1: build
# ────────────────────────────────────────────────────────────────────
FROM rust:1.95-alpine AS builder

RUN apk add --no-cache musl-dev git

WORKDIR /src

# Copy the manifest first so the dependency compile layer caches across
# source edits.
COPY Cargo.toml Cargo.lock ./
# Ensure the dummy main.rs doesn't need real source imports.
RUN mkdir -p src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && rm -rf src

COPY . .
# Touch main.rs so cargo re-compiles our code (not just deps).
RUN touch src/main.rs \
    && cargo build --release --target x86_64-unknown-linux-musl \
    && strip target/x86_64-unknown-linux-musl/release/tok0

# ────────────────────────────────────────────────────────────────────
# Stage 2: runtime
# ────────────────────────────────────────────────────────────────────
FROM alpine:3.20

# git is commonly needed by the commands tok0 proxies. Add coreutils
# for POSIX-compliant ls/grep behaviour (alpine ships BusyBox variants).
RUN apk add --no-cache git coreutils ca-certificates

COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/tok0 /usr/local/bin/tok0

# Non-root default user for security
RUN addgroup -S tok0 && adduser -S tok0 -G tok0
USER tok0
WORKDIR /work

ENTRYPOINT ["/usr/local/bin/tok0"]
CMD ["--help"]
