FROM rust:1-buster AS builder

RUN apt-get update && apt-get -y upgrade && apt-get -y install git openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin patreon-proxy

# Prod container
FROM debian:buster

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=builder /tmp/compile/target/release/patreon-proxy /srv/proxy/proxy
RUN chmod +x /srv/proxy/proxy

RUN useradd -m container
USER container
WORKDIR /srv/proxy

CMD ["/srv/proxy/proxy"]