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

#[async_trait] // Swaps around pronouns in the input
impl MessageMutator for Misgendering {
    async fn mutate(&self, input: String, _: &Context, _: &SclunerGuild) -> Option<String> {
        if !rng().random_ratio(1, 9) { return None }

        const VALID_PRONOUNS: [&str; 4] = [
            "he",
            "she",
            "it",
            "they"
        ];

        const VALID_PRONOUNS_OTHER: [&str; 4] = [
            "him",
            "her",
            "it",
            "them"
        ];

        const VALID_PRONOUNS_OWNING: [&str; 4] = [
            "his",
            "her",
            "its",
            "their"
        ];

        const VALID_CHECK: [&str; 12] = [
            "he",
            "she",
            "it",
            "they",

            "him",
            "her",
            "it",
            "them",

            "his",
            "her",
            "its",
            "their"
        ];

        let mut any_contained = false;
        for p in VALID_CHECK {
            if input.contains(p) {
                any_contained = true;
                break;
            }
        };

        if !any_contained {
            return None;
        }

        let mut tokens = input.split_whitespace();
        let mut unchanged = 0;
        let mut output = String::new();

        while unchanged <= 10 {
            for token in &mut tokens {
                let token_lowercase = token.to_lowercase();
                let token_lowercase_str = token_lowercase.as_str();

                let valid_pronoun = VALID_PRONOUNS.contains(&token_lowercase_str);
                let valid_owning_pronoun = VALID_PRONOUNS_OWNING.contains(&token_lowercase_str);
                let valid_other_pronoun = VALID_PRONOUNS_OTHER.contains(&token_lowercase_str);

                let mut token_add = token;
                if (valid_pronoun || valid_owning_pronoun || valid_other_pronoun) && rng().random_ratio(1, 3) {
                    
                    if valid_pronoun {
                        token_add = VALID_PRONOUNS.choose(&mut rng()).unwrap();
                    }
                    else if valid_owning_pronoun {
                        token_add = VALID_PRONOUNS_OWNING.choose(&mut rng()).unwrap();
                    }
                    else if valid_other_pronoun {
                        token_add = VALID_PRONOUNS_OTHER.choose(&mut rng()).unwrap();
                    }

                    unchanged = 11;
                    continue;
                }

                output += token_add;
                output += " ";
            }
        }

        Some(output)
    }
}