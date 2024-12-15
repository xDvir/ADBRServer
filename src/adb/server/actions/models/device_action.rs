use super::action::Action;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceActions {
    pub connect: Option<Vec<Action>>,
    pub disconnect: Option<Vec<Action>>,
}
