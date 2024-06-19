FROM rust:1-buster

RUN apt-get install -y apt-transport-https
RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin whitelabel --no-default-features --features whitelabel,skip-initial-guild-creates,use-sentry,metrics

FROM debian:buster

RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates tini

COPY --from=0 /tmp/compile/target/release/whitelabel /srv/sharder/sharder
RUN chmod +x /srv/sharder/sharder

RUN useradd -m container
USER container
WORKDIR /srv/sharder

COPY ./sharder/entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/bin/bash", "/entrypoint.sh"]