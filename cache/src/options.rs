#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub users: bool,
    pub guilds: bool,
    pub members: bool,
    pub channels: bool,
    pub threads: bool,
    pub roles: bool,
    pub emojis: bool,
    pub voice_states: bool,
}

impl Options {
    pub fn new(
        users: bool,
        guilds: bool,
        members: bool,
        channels: bool,
        threads: bool,
        roles: bool,
        emojis: bool,
        voice_states: bool,
    ) -> Options {
        Options {
            users,
            guilds,
            members,
            channels,
            threads,
            roles,
            emojis,
            voice_states,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            users: true,
            guilds: true,
            members: true,
            channels: true,
            threads: true,
            roles: true,
            emojis: true,
            voice_states: true,
        }
    }
}
