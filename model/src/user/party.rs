use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Party {
    pub id: Option<String>,
    pub size: Option<[u32; 2]>, // [current_size, max_size]
}
