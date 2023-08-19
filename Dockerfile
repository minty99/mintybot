FROM rust:1.71.1

WORKDIR /usr/src/mintybot
COPY . .

RUN cargo build --release

CMD ["./target/release/mintybot"]