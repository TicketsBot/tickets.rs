use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Party {
    pub id: Option<String>,
    pub size: Option<[u32; 2]>, // [current_size, max_size]
}
