FROM rustlang/rust:nightly as builder

WORKDIR /app
COPY . .

# Build with release optimizations
RUN cargo build --release

# Create a smaller runtime image
FROM debian:bookworm-slim

# Install SSL certificates and other runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    update-ca-certificates

WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/mintybot /app/mintybot

# Set environment variables (will be overridden by docker-compose)
ENV MINTYBOT_DISCORD_TOKEN=""
ENV MINTYBOT_OPENAI_TOKEN=""
ENV MINTYBOT_DEV_USER_ID=""

# Run the binary
CMD ["./mintybot"]