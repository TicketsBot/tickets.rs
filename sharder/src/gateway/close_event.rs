#[derive(Debug)]
pub struct CloseEvent {
    pub status_code: u16,
    pub error: String,
}

impl CloseEvent {
    pub fn new(status_code: u16, error: String) -> Self {
        Self { status_code, error }
    }

    pub fn should_reconnect(&self) -> bool {
        !matches!(self.status_code, 4004 | 4010 | 4011 | 4012 | 4013 | 4014)
    }
}
