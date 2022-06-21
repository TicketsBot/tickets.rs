#[derive(Clone, Copy)]
pub struct Options {
    pub users: bool,
    pub guilds: bool,
    pub members: bool,
    pub channels: bool,
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
        roles: bool,
        emojis: bool,
        voice_states: bool,
    ) -> Options {
        Options {
            users,
            guilds,
            members,
            channels,
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
            roles: true,
            emojis: true,
            voice_states: true,
        }
    }
}
