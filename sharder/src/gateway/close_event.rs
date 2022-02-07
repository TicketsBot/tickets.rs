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
        match self.status_code {
            4004 | 4010 | 4011 | 4012 | 4013 | 4014 => false,
            _ => true,
        }
    }
}
