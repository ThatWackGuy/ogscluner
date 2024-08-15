mod scluner_backup;
mod commands;

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use async_std::sync::{Mutex};
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::*;
use rand::prelude::*;
use rand::thread_rng;
use shuttle_runtime::{SecretStore};
use serde::{Deserialize, Serialize};
use crate::scluner_backup::SclunerBackup;
use crate::commands::*;

type Error = Box<dyn std::error::Error + Send + Sync>;
type SclunerRef = Arc<Mutex<SclunerInstance>>;
type Context<'a> = poise::Context<'a, SclunerRef, Error>;
type DataContext<'a> = poise::PrefixContext<'a, SclunerRef, Error>;

#[derive(Serialize, Deserialize, Clone)]
struct SclunerMessage {
    user_id: UserId,
    content: String
}

impl SclunerMessage {
    fn new(msg: &Message) -> Self {
        Self {
            user_id: msg.author.id,
            content: msg.content.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct SclunerGuild {
    guild_id: GuildId,
    messages: Vec<SclunerMessage>,
    asleep: bool,

    min_proc: u32,
    max_proc: u32,
    proc_out_of: u32,

    proc: u32,
}

impl SclunerGuild {
    fn new(guild_id: GuildId) -> Self {
        println!("NEW GUILD REGISTERED: {}", guild_id);

        Self {
            guild_id,
            messages: Vec::new(),
            asleep: false,

            min_proc: 1,
            max_proc: 4,
            proc_out_of: 18,

            proc: thread_rng().gen_range(1..4)
        }
    }

    async fn send_random(&mut self, ctx: &serenity::Context, channel_id: ChannelId) {
        let messages = &self.messages;

        let mut keep_going = true;
        while keep_going {
            // fake typing
            let typing = channel_id.start_typing(&ctx.http);

            keep_going = thread_rng().gen_ratio(1, 4);

            let mut message = match messages.choose(&mut thread_rng()) {
                None => {
                    eprintln!("FAILED TO SEND RANDOM RESPONSE: NO RECORDED MESSAGES");
                    return;
                }
                Some(m) => m.content.clone()
            };

            if keep_going { message += " ..." }
            async_std::task::sleep(Duration::from_millis((100 * message.split_whitespace().count()) as u64)).await;

            if let Err(e) = channel_id.say(ctx.http(), message).await {
                eprintln!("FAILED TO SEND RANDOM RESPONSE: {}", e);
            }
            typing.stop();
        }
    }

    async fn maybe_react_random(&mut self, ctx: &serenity::Context, msg: &Message) {
        let emojis = self.guild_id.emojis(ctx.http()).await.unwrap();

        let mut keep_going = thread_rng().gen_ratio(1, 8);
        while keep_going {
            keep_going = thread_rng().gen_ratio(1, 4);

            let emote = emojis.choose(&mut thread_rng()).unwrap();

            if let Err(e) = msg.react(ctx.http(), ReactionType::from(emote.clone())).await {
                eprintln!("FAILED TO REACT: {}", e);
            }
        }
    }

    fn fetch_from_content(&mut self, content: String) -> Vec<&SclunerMessage> {
        self.messages.iter().filter(|m| m.content.contains(&content)).collect()
    }

    fn delete_message_sender(&mut self, user_id: UserId) {
        self.messages.retain(|m| m.user_id != user_id);
    }

    fn delete_message_content(&mut self, content: String) {
        self.messages.retain(|m| !m.content.contains(content.as_str()));
    }
}

struct SclunerInstance {
    startup_instant: Instant,
    backup_instant: Instant,
    guilds: HashMap<GuildId, SclunerGuild>,
    whitelist: Vec<UserId>,
    blacklist: Vec<UserId>,
    modlist: Vec<UserId>
}
impl SclunerInstance {
    fn new() -> Self {
        Self {
            startup_instant: Instant::now(),
            backup_instant: Instant::now(),
            guilds: HashMap::new(),
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            modlist: Vec::new()
        }
    }

    async fn save_backup(&self, ctx: &serenity::Context) {
        let backup = SclunerBackup::new(&self.guilds, &self.whitelist, &self.blacklist, &self.modlist);

        let mut data: Vec<u8> = Vec::new();
        if let Err(e) = ciborium::into_writer(&backup, &mut data) {
            eprintln!("FAILED TO BACKUP : SERIALISATION UNSUCCESSFUL:{}", e);
            return;
        }

        // private channel
        if let Err(e) = ChannelId::from(970308154401378356).send_files(ctx.http(), vec![CreateAttachment::bytes(data, "backup.cbor")], Default::default()).await {
            eprintln!("FAILED TO BACKUP : FILES COULDN'T BE SENT:{}", e);
            return;
        }

        println!("BACKUP SUCCESSFUL!");
    }

    fn load_backup(&mut self, load: SclunerBackup) {
        let guilds = load.guilds_keys.into_iter().zip(load.guilds_values).collect();

        self.backup_instant = Instant::now();
        self.guilds = guilds;
        self.whitelist = load.whitelist;
        self.blacklist = load.blacklist;
        self.modlist = load.modlist;
    }
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, SclunerRef, Error>,
    data: &SclunerRef,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot: bot, .. } => {
            println!("HEY {}!", bot.user.name.to_ascii_uppercase());
        }

        FullEvent::Message { new_message: msg } => {
            if msg.author.bot { return Ok(()); }
            if msg.content.contains("/unscule") || msg.content.contains("::SCL_") { return Ok(()); }
            let guild_id = match msg.guild_id {
                None => return Ok(()),
                Some(g) => g
            };
            let mut data = data.lock().await;

            // auto backup per 12 hours
            if data.backup_instant.elapsed().as_secs() >= 43200 {
                data.save_backup(ctx).await;
                data.backup_instant = Instant::now();
            }

            let whitelisted = data.whitelist.contains(&msg.author.id);
            let blacklisted = data.blacklist.contains(&msg.author.id);
            let guild = match data.guilds.get_mut(&guild_id) {
                None => {
                    data.guilds.insert(guild_id, SclunerGuild::new(guild_id));
                    data.guilds.get_mut(&guild_id).unwrap()
                }
                Some(g) => g
            };

            if guild.asleep { return Ok(()) }

            guild.maybe_react_random(ctx, msg).await;

            // reply if we procced, or they're pinging it or replying to it
            if thread_rng().gen_ratio(guild.proc, guild.proc_out_of) || msg.mentions_me(ctx.http()).await.unwrap() {
                guild.send_random(ctx, msg.channel_id).await;
            }

            if msg.mentions.is_empty()&& !blacklisted && whitelisted && !msg.content.is_empty() && msg.content.len() < 2000 && msg.content.split_whitespace().count() < 30 {
                guild.messages.push(SclunerMessage::new(msg));

                // message limit
                if guild.messages.len() > 2222 {
                    let remove_idx = thread_rng().gen_range(1000..=2222);
                    guild.messages.swap_remove(remove_idx);
                }
            }
        }
        _ => {}
    }

    Ok(())
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_serenity::ShuttleSerenity {
    let token = secrets.get("DISCORD_TOKEN").expect("'DISCORD_TOKEN' was not found");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions  {
            commands: vec![
                // USER
                delete_content(),
                info_content(),
                info_proc(),
                info(),

                // MODS
                delete_user(),
                proc(),
                sleep(),

                // DEV
                moderator(),
                whitelist(),
                backup_send(),
                backup_load()
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("::SCL_".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                Ok(Arc::new(Mutex::from(SclunerInstance::new())))
            })
        })
        .build();

    let client = Client::builder(&token, intents)
        .framework(framework)
        .await
        .expect("Failed to create client!");

    Ok(client.into())
}
