use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub struct Options {
    #[serde(default)]
    pub users: bool,
    #[serde(default)]
    pub guilds: bool,
    #[serde(default)]
    pub members: bool,
    #[serde(default)]
    pub channels: bool,
    #[serde(default)]
    pub roles: bool,
    #[serde(default)]
    pub emojis: bool,
    #[serde(default)]
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
