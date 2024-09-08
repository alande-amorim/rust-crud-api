# build stage
FROM rust:1.70-buster as builder

WORKDIR /app
ARG DATABASE_URL
ENV DATABASE_URL=$DATABASE_URL
COPY . .
RUN cargo build --release

# production stage
FROM debian:buster-slim
WORKDIR /usr/local/bin
COPY --from=builder /app/target/release/crud-api .
CMD ["./crud-api"]