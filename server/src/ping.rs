use protocol::packet_id::CURRENT_MC_PROTOCOL;
use serde::Serialize;
use uuid::Uuid;
use valence_text::{Color, IntoText, Text};

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ServerListPing {
    Respond {
        version: Version,
        players: Players,
        #[serde(rename = "description")]
        desc: Text,
        favicon: Option<String>,
    },
    Ignore,
}

impl Default for ServerListPing {
    fn default() -> Self {
        Self::Respond {
            version: Version::default(),
            players: Players::default(),
            desc: "A Minecraft Server".color(Color::GRAY),
            favicon: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Players {
    online: i32,
    max: i32,
    sample: Vec<PlayerSampleEntry>,
}

impl Default for Players {
    fn default() -> Self {
        Self {
            online: 0,
            max: 20,
            sample: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerSampleEntry {
    pub name: String,
    pub id: Uuid,
}

#[derive(Clone, Debug, Serialize)]
pub struct Version {
    name: String,
    protocol: i32,
}

impl Default for Version {
    fn default() -> Self {
        Self {
            name: "1.21.5".to_string(),
            protocol: CURRENT_MC_PROTOCOL as i32,
        }
    }
}
