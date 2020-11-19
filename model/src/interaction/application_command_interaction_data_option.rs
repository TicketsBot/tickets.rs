use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandInteractionDataOption {
    pub name: Box<str>,
    // pub value: Option<OptionType>, TODO: Implement OptionType when documented
    pub options: Option<Vec<ApplicationCommandInteractionDataOption>>,
}
