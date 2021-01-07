pub enum PodUpdate {
    Add(PodData),
    Remove(String), // pod_name
    LostConnection(Vec<PodData>), // new_pods
}

pub struct PodData {
    pub name: String,
    pub ip: String,
}

impl PodData {
    pub fn new(name: String, ip: String) -> PodData {
        PodData { name, ip }
    }
}