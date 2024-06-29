use clap::Parser;
use hmac::digest::KeyInit;
use hmac::Hmac;
use jwt::SignWithKey;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::ops::Add;
use std::time::{Duration, SystemTime};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    key: String,

    #[clap(short, long, value_parser)]
    url: String,
}

fn main() {
    let args = Args::parse();

    let key: Hmac<Sha256> =
        Hmac::new_from_slice(args.key.as_bytes()).expect("Failed to parse HMAC key");

    let mut claims = BTreeMap::new();
    claims.insert("url", args.url);
    claims.insert("request_id", uuid::Uuid::new_v4().to_string());
    claims.insert(
        "exp",
        SystemTime::now()
            .add(Duration::from_secs(60))
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string(),
    );

    let token = claims.sign_with_key(&key).unwrap();
    println!("{token}");
}
