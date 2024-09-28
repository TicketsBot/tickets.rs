FROM rust:1-buster

RUN apt-get install -y apt-transport-https
RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin cache-sync-service

FROM debian:buster

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=0 /tmp/compile/target/release/cache-sync-service /srv/cache-sync-service/cache-sync-service
RUN chmod +x /srv/cache-sync-service/cache-sync-service

RUN useradd -m container
USER container
WORKDIR /srv/cache-sync-service

CMD ["/srv/cache-sync-service/cache-sync-service"]