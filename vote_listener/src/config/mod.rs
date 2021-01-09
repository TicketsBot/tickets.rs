use std::env;

pub struct Config {
    pub server_addr: Box<str>,
    pub dbl_signature: Box<str>,
    pub database_uri: Box<str>,
    pub vote_url: Box<str>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            server_addr: var_or_panic("SERVER_ADDR"),
            dbl_signature: var_or_panic("DBL_TOKEN"),
            database_uri: var_or_panic("DATABASE_URI"),
            vote_url: var_or_panic("VOTE_URL"),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

fn var_or_panic(name: &str) -> Box<str> {
    Box::from(env::var(name).unwrap())
}