FROM rust:1 AS builder
COPY . /src
WORKDIR /src/ociarchive2disk
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:latest
COPY --from=builder /src/ociarchive2disk/target/release/ociarchive2disk /
ENTRYPOINT ["/ociarchive2disk"]
