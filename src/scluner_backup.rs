use std::collections::HashMap;
use poise::serenity_prelude::*;
use serde::{Deserialize, Serialize};
use crate::{SclunerGuild, SclunerMessage};
use crate::mutators::DefinedMutators;

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

// BACKUP COMPAT
// Update these every time SclunerGuild or SclunerBackup changes
// SclunerMessage is not expected to change
// 2.0.0 -> 3.0.0

#[derive(Deserialize, Clone)]
pub struct SclunerGuildCompat {
    guild_id: GuildId,
    messages: Vec<SclunerMessage>,
    asleep: bool,

    min_proc: u32,
    max_proc: u32,
    proc_out_of: u32,

    proc: u32,
}

impl SclunerGuildCompat {
    pub fn modernise(self) -> SclunerGuild {
        SclunerGuild{
            guild_id: self.guild_id,
            messages: self.messages,
            asleep: self.asleep,

            allowed_mutators: DefinedMutators::default_allowed(),

            min_proc: self.min_proc,
            max_proc: self.max_proc,
            proc_out_of: self.proc_out_of,

            proc: self.proc,
        }
    }
}

#[derive(Deserialize)]
pub struct SclunerBackupCompat {
    pub guilds_keys: Vec<GuildId>,
    pub guilds_values: Vec<SclunerGuildCompat>,
    pub whitelist: Vec<UserId>,
    pub blacklist: Vec<UserId>,
    pub modlist: Vec<UserId>
}

impl SclunerBackupCompat {
    pub fn modernise(self) -> SclunerBackup {
        SclunerBackup{
            guilds_keys: self.guilds_keys,
            guilds_values: self.guilds_values.iter().map(move |g| { g.clone().modernise() }).collect(),
            whitelist: self.whitelist,
            blacklist: self.blacklist,
            modlist: self.modlist,
        }
    }
}