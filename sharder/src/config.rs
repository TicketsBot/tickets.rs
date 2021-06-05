use crate::{get_worker_svc_uri, var_or_panic};

pub struct Config {
    pub sticky_cookie: Box<str>,
    pub worker_svc_uri: Box<str>,
}

impl Config {
    pub fn from_envvar() -> Config {
        Config {
            sticky_cookie: var_or_panic("WORKER_STICKY_COOKIE"),
            worker_svc_uri: get_worker_svc_uri(),
        }
    }
}
