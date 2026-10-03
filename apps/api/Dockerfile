FROM rust:1-slim-bookworm AS build
WORKDIR /app
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/api /usr/local/bin/api
COPY --from=build /app/target/release/create_admin /usr/local/bin/create_admin
EXPOSE 8080
CMD ["api"]
