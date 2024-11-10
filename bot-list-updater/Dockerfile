FROM rust:1-buster

RUN apt-get install -y apt-transport-https
RUN apt-get update && apt-get -y upgrade && apt-get -y install python3 openssl libssl-dev ca-certificates cmake

RUN mkdir -p /tmp/compile
WORKDIR /tmp/compile

COPY . .

RUN cargo build --release --bin bot_list_updater

FROM debian:buster

RUN apt-get update && apt-get -y upgrade && apt-get -y install openssl libssl-dev ca-certificates

COPY --from=0 /tmp/compile/target/release/bot_list_updater /srv/bot-list-updater/bot-list-updater
RUN chmod +x /srv/bot-list-updater/bot-list-updater

RUN useradd -m container
USER container
WORKDIR /srv/bot-list-updater

CMD ["/srv/bot-list-updater/bot-list-updater"]