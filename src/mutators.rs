use std::sync::Arc;

use poise::serenity_prelude::*;
use rand::prelude::SliceRandom;
use rand::{Rng, thread_rng};

use crate::SclunerGuild;

pub type MutatorRef = Arc<dyn MessageMutator + Send + Sync>;
#[async_trait]
pub trait MessageMutator: Send + Sync {
    async fn mutate(&self, input: String, ctx: &Context, guild: &SclunerGuild) -> Option<String>;
}

// DEFINITIONS
pub struct AppendEmote;

// IMPLEMENTATIONS
#[async_trait]
impl MessageMutator for AppendEmote {
    async fn mutate(&self, input: String, ctx: &Context, guild: &SclunerGuild) -> Option<String> {
        if !thread_rng().gen_ratio(1, 16) { return None }

        let emotes = guild.guild_id.emojis(ctx.http()).await.unwrap();
        let emote = emotes.choose(&mut thread_rng()).unwrap();

        Some(format!("{} {}", input, emote))
    }
}