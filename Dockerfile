FROM docker.io/rust:slim AS builder

RUN apt-get update -y && apt-get install -y musl-tools
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
	"linux/amd64") echo x86_64-unknown-linux-musl > /rust_targets ;; \
	*) exit 1 ;; \
    esac

RUN rustup target add $(cat /rust_targets)

COPY . ./exporter
WORKDIR /exporter
RUN cargo build --release --target $(cat /rust_targets)
RUN strip --strip-all target/release/findora-exporter

FROM docker.io/busybox:latest

COPY --from=builder /exporter/target/release/findora-exporter /exporter

EXPOSE 9090
ENTRYPOINT ["/exporter", "--config", "/config.json"]
