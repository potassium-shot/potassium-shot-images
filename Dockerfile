FROM rust:1.99.1-slim AS builder
WORKDIR /usr/src/potassium-shot-images
COPY . .
RUN cargo build --release

FROM debian:oldstable-20260316-slim
COPY --from=builder /usr/src/potassium-shot-images/target/release/potassium-shot-images /usr/local/bin
CMD [ "/usr/local/bin/potassium-shot-images" ]
