FROM rust:1-slim AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

# Minimal runtime: no shell, no keys, no state.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /src/target/release/echo-merchant /echo-merchant
# Listen address; override with -e ECHO_MERCHANT_ADDR=0.0.0.0:PORT
ENV ECHO_MERCHANT_ADDR=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["/echo-merchant"]
