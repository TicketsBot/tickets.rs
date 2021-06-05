pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: i64, // seconds since epoch
}

impl Tokens {
    pub fn new(access_token: String, refresh_token: String, expires: i64) -> Tokens {
        Tokens {
            access_token,
            refresh_token,
            expires,
        }
    }
}
