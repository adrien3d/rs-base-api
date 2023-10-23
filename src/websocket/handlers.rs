use crate::ProgramAppState;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BasicCommandData {
    /// Browser timestamp
    pub timestamp: u16,
    /// The target of the command.
    pub target: u8,
    /// The actual command.
    pub command: u8,
}

pub fn basic_command(
    _state: &web::Data<ProgramAppState>,
    data: BasicCommandData,
) -> anyhow::Result<()> {
    log::info!(
        "Sending a Basic command at time: browser:{} server:{} to target:{}, with command:{}",
        data.timestamp,
        data.timestamp,
        data.target,
        data.command
    );
    Ok(())
}
