use std::collections::HashMap;
use poise::serenity_prelude::*;
use serde::{Deserialize, Serialize};
use crate::SclunerGuild;

#[derive(Serialize, Deserialize)]
pub struct SclunerBackup {
    pub guilds_keys: Vec<GuildId>,
    pub guilds_values: Vec<SclunerGuild>,
    pub whitelist: Vec<UserId>,
    pub blacklist: Vec<UserId>,
    pub modlist: Vec<UserId>
}

impl SclunerBackup {
    pub fn new(guilds: &HashMap<GuildId, SclunerGuild>, whitelist: &Vec<UserId>, blacklist: &Vec<UserId>, modlist: &Vec<UserId>) -> Self {
        Self {
            guilds_keys: guilds.keys().cloned().collect(),
            guilds_values: guilds.values().cloned().collect(),
            whitelist: whitelist.clone(),
            blacklist: blacklist.clone(),
            modlist: modlist.clone(),
        }
    }
}