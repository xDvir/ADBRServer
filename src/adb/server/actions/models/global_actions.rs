use super::action::Action;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalActions {
    pub connect: Vec<Action>,
    pub disconnect: Vec<Action>,
}