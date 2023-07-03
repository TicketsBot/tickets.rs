# syntax = docker/dockerfile:experimental
FROM rustlang/rust:nightly-buster

RUN apt-get install -y apt-transport-https
RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates cmake
RUN cargo install sccache

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN --mount=type=cache,target=/root/.cache/sccache cargo +nightly build --release --bin public

FROM debian:buster

RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates tini

COPY --from=0 /tmp/compile/target/release/public /srv/sharder/sharder
RUN chmod +x /srv/sharder/sharder

RUN useradd -m container
USER container
WORKDIR /srv/sharder

COPY ./entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/bin/bash", "/entrypoint.sh"]