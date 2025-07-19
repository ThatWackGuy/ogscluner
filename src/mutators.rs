use std::sync::Arc;

use poise::serenity_prelude::*;
use rand::prelude::IndexedRandom;
use rand::{rng, Rng};
use serde::{Serialize, Deserialize};

use crate::SclunerGuild;

pub type MutatorRef = Arc<dyn MessageMutator + Send + Sync>;

#[async_trait]
pub trait MessageMutator: Send + Sync {
    async fn mutate(&self, input: String, ctx: &Context, guild: &SclunerGuild) -> Option<String>;
}

#[derive(Serialize, Deserialize, Clone)]
pub enum DefinedMutators {
    AppendEmote,
    MessageSplicer,
    Misgendering
}

impl DefinedMutators {
    pub fn default_allowed() -> Vec<Self> {
        vec![
            Self::AppendEmote,
            Self::MessageSplicer,
            Self::Misgendering
        ]
    }

    pub fn to_mutators(allowed: &Vec<Self>) -> Vec<MutatorRef> {
        allowed.iter().map(|mutator| {
            match mutator {
                DefinedMutators::AppendEmote => Arc::new(AppendEmote) as MutatorRef,
                DefinedMutators::MessageSplicer => Arc::new(MessageSplicer) as MutatorRef,
                DefinedMutators::Misgendering => Arc::new(Misgendering) as MutatorRef,
            }
        }).collect()
    }
}

// DEFINITIONS
pub struct AppendEmote;
pub struct MessageSplicer;
pub struct Misgendering;

// IMPLEMENTATIONS
#[async_trait] // Appends an emote at the end of the input
impl MessageMutator for AppendEmote {
    async fn mutate(&self, input: String, ctx: &Context, guild: &SclunerGuild) -> Option<String> {
        if !rng().random_ratio(1, 16) { return None }

        let emotes = guild.guild_id.emojis(ctx.http()).await.unwrap();
        let emote = emotes.choose(&mut rng()).unwrap();

        Some(format!("{} {}", input, emote))
    }
}

// TODO: Improve message splicer
#[async_trait] // Splices the input and another message together
impl MessageMutator for MessageSplicer {
    async fn mutate(&self, input: String, _: &Context, guild: &SclunerGuild) -> Option<String> {
        if !rng().random_ratio(1, 16) { return None }

        let input_tokens = input.split_whitespace();
        let random_tokens = guild.messages.choose(&mut rng()).unwrap().content.split_whitespace();

        let input_len = input.len();
        let random_len = random_tokens.clone().count();

        let splicing_input = input_len > random_len;

        // Take most from input if its larger
        if splicing_input {
            let input_range = rng().random_range(0..input_len);
            let input_slice = input_tokens.take(input_range).collect::<Vec<&str>>().join(" ");
            let random_slice = random_tokens.skip(input_range).collect::<Vec<&str>>().join(" ");

            return Some(input_slice + random_slice.as_str())
        }

        let random_range = rng().random_range(0..random_len);
        let random_slice = random_tokens.take(random_range).collect::<Vec<&str>>().join(" ");
        let input_slice = input_tokens.skip(random_range).collect::<Vec<&str>>().join(" ");

        Some(random_slice + input_slice.as_str())
    }
}

pub struct Pronouns<'a> {
    pub primary: [&'a str; 4],
    pub other: [&'a str; 4],
    pub owning: [&'a str; 4],
    pub all: [&'a str; 12],
}

// Internals for misgendering mutator
impl Misgendering {
    const PRONOUNS: Pronouns<'static> = Pronouns {
        primary: ["he", "she", "it", "they"],
        other: ["him", "her", "it", "them"],
        owning: ["his", "her", "its", "their"],
        all: [
            "he", "she", "it", "they", "him", "her", "it", "them", "his", "her", "its", "their",
        ],
    };

    fn randomize_pronoun(input: Vec<&str>, idx: usize) -> Vec<&str> {
        let mut output: Vec<&str> = input.clone();
        if Self::PRONOUNS.primary.contains(&input[idx]) {
            output[idx] = Self::PRONOUNS.primary.choose(&mut rng()).unwrap();
        } else if Self::PRONOUNS.other.contains(&input[idx]) {
            output[idx] = Self::PRONOUNS.other.choose(&mut rng()).unwrap();
        } else if Self::PRONOUNS.owning.contains(&input[idx]) {
            output[idx] = Self::PRONOUNS.owning.choose(&mut rng()).unwrap();
        }
        return output;
    }
}

#[async_trait] // Swaps around pronouns in the input
impl MessageMutator for Misgendering {
    async fn mutate(&self, input: String, _: &Context, _: &SclunerGuild) -> Option<String> {
        if !rng().random_ratio(1, 9) {
            return None;
        }

        let any_contained: bool = input.split_whitespace().any(|e| {
            Misgendering::PRONOUNS
                .all
                .contains(&e.to_lowercase().as_str())
        });

        if !any_contained {
            return None;
        }

        let mut new_tokens: Vec<&str> = input.split_whitespace().collect();

        // Save indexes where woke
        let mut pronoun_indexes: Vec<usize> = Vec::new();

        for (i, token) in new_tokens.iter().enumerate() {
            let token_lowercase = token.to_lowercase();
            let token_lowercase_str = token_lowercase.as_str();

            if Misgendering::PRONOUNS.all.contains(&token_lowercase_str) {
                pronoun_indexes.push(i);
            }
        }

        // Make sure at least one pronoun is woked
        let guaranteed_idx = pronoun_indexes
            .swap_remove(pronoun_indexes[rng().random_range(0..pronoun_indexes.len())]);
        new_tokens = Misgendering::randomize_pronoun(new_tokens, guaranteed_idx);

        // 3/4 chance for each one to :3
        for to_change_idx in pronoun_indexes {
            if rng().random_ratio(3, 4) {
                new_tokens = Misgendering::randomize_pronoun(new_tokens, to_change_idx);
            }
        }

        Some(new_tokens.join(" "))
    }
}
