FROM rust:1-buster AS builder

RUN apt-get update && apt-get -y upgrade && apt-get -y install git openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin stats-channel-updater

# Prod container
FROM debian:buster

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=builder /tmp/compile/target/release/stats-channel-updater /srv/stats-channel-updater/stats-channel-updater
RUN chmod +x /srv/stats-channel-updater/stats-channel-updater

RUN useradd -m container
USER container
WORKDIR /srv/stats-channel-updater

CMD ["/srv/stats-channel-updater/stats-channel-updater"]