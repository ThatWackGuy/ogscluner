use std::fs;
use std::io::{Read, Write};
use std::ops::Add;
use std::str::FromStr;
use rand::prelude::*;
use regex::Regex;
use serenity::all::*;
use serenity::futures::StreamExt;
use serenity::prelude::*;
use shuttle_runtime::__internals::Context;
use shuttle_runtime::SecretStore;

struct SavedMessage {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub content: String,

    messages_path: String,
    users_path: String
}

impl SavedMessage {
    pub fn new(msg: Message, guild_id: GuildId) -> Self {
        let messages_path = format!("./{}_msg", guild_id);
        let users_path = format!("./{}_id", guild_id);

        if let Err(_) = fs::metadata(&messages_path) { fs::write(&messages_path, "").unwrap() }
        if let Err(_) = fs::metadata(&users_path) { fs::write(&users_path, "").unwrap() }

        return Self {
            guild_id,
            user_id: msg.author.id,
            content: msg.content,

            messages_path: format!("./{}_msg", guild_id),
            users_path: format!("./{}_id", guild_id),
        }
    }

    pub fn find_from_message(content: String, guild_id: GuildId) -> Option<Self> {
        let messages_path = format!("./{}_msg", guild_id);
        let users_path = format!("./{}_id", guild_id);

        let msg_file = fs::read_to_string(&messages_path).unwrap();
        let mut messages = msg_file.split("\n");
        let msg_index = match messages.position(|x| x == content) {
            None => return None,
            Some(idx) => idx
        };

        println!("FETCHING LINE: {}", msg_index);

        let message = messages.nth(msg_index).unwrap();
        let file_users = fs::read_to_string(&users_path).unwrap();
        let user = file_users.split("\n").nth(msg_index).unwrap();

        Some(Self {
            guild_id,
            user_id: UserId::from_str(user).unwrap(),
            content:  message.to_string(),

            messages_path,
            users_path
        })
    }

    fn fetch_random(guild_id: GuildId) -> String {
        let file_msg = fs::read_to_string(format!("./{}_msg", guild_id)).unwrap();
        file_msg.split("\n").choose(&mut thread_rng()).unwrap().to_string()
    }

    fn save(&self) {
        let mut file_msg = fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(&self.messages_path)
            .unwrap();

        let mut file_ids = fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(&self.users_path)
            .unwrap();

        if let Err(e) = writeln!(file_msg, "{}", self.content) {
            eprintln!("COULDN'T ADD MESSAGE TO FILE: {}", e);
        }

        if let Err(e) = writeln!(file_ids, "{}", self.user_id) {
            eprintln!("COULDN'T ADD MESSAGE USER ID TO FILE: {}", e);
        }
    }

    fn delete(&self) -> Result<()> {
        let file_msg = fs::read_to_string(&self.messages_path)?;
        let file_ids = fs::read_to_string(&self.users_path)?;

        let mut messages = file_msg.split("\n");
        let ids = file_ids.split("\n");

        // get index of message
        let index = match messages.position(|s| s == self.content) {
            None => Err(Error::)
            Some(_) => {}
        };

        // skip the index of deleted message
        let deleted_msg: Vec<&str> = messages.enumerate().filter_map(|(i, e)| if i != index { Some(e) } else { None }).collect();
        let deleted_ids: Vec<&str> = ids.enumerate().filter_map(|(i, e)| if i != index { Some(e) } else { None }).collect();

        let msg_overwrite = deleted_msg.join("\n");
        let id_overwrite = deleted_ids.join("\n");

        // overwrite file
        fs::write(&self.messages_path, msg_overwrite)?;
        fs::write(&self.users_path, id_overwrite)?;

        Ok(())
    }
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: prelude::Context, msg: Message) {
        if msg.author.bot { return; }
        let guild_id = match msg.guild_id {
            None => return,
            Some(id) => id
        };

        // skip any messages with mentions
        if msg.kind != MessageType::InlineReply && msg.mentions.len() > 0 {
            // ogscluner mention is special as it adds you to the whitelist
            match msg.mentions.iter().find(|u| u.id == 1268981775158476982) {
                None => {}
                Some(_) => {
                    println!("ADDING {} TO THE WHITELIST..", msg.author.name);

                    let wh_file = fs::read_to_string("./whitelist").unwrap();
                    if wh_file.split("\n").any(|u| u == msg.author.id.to_string()) { return; }

                    let mut whitelist = fs::OpenOptions::new()
                        .read(true)
                        .append(true)
                        .open("./whitelist")
                        .unwrap();

                    if let Err(e) = writeln!(whitelist, "{}", msg.author.id) {
                        eprintln!("COULDN'T ADD USER TO WHITELIST: {}", e);
                    }
                }
            }

            return;
        }

        // skip any message mentioning the scule
        if let Some(_) = Regex::new("scluner").unwrap().find(msg.content.as_str()) { return; }

        let save_msg = SavedMessage::new(msg.clone(), guild_id);

        if msg.kind == MessageType::InlineReply && msg.mentions[0].id == 1268981775158476982 {
            let ref_msg = msg.clone().referenced_message.unwrap();

            // ELSE IF HELL
            // I DON'T CARE ENOUGH TO FIX

            // LOAD MESSAGES INTO FILE
            if msg.content.starts_with("::SCL_LOAD_MSG") {
                if msg.author.id != 407991620164911118 { return; }

                fs::write(save_msg.messages_path, msg.content.replace("::SCL_LOAD_MSG\n", "")).unwrap();
                msg.channel_id.say(ctx.http(), "MANUAL MESSAGE LOAD SUCCESSFUL").await.unwrap();

                return;
            }
            // LOAD USERS INTO FILE
            else if msg.content.starts_with("::SCL_LOAD_ID") {
                if msg.author.id != 407991620164911118 { return; }

                fs::write(save_msg.users_path, msg.content.replace("::SCL_LOAD_ID\n", "")).unwrap();
                msg.channel_id.say(ctx.http(), "MANUAL ID LOAD SUCCESSFUL").await.unwrap();

                return;
            }
            // LOAD USERS INTO FILE
            else if msg.content.starts_with("::SCL_LOAD_WHITELIST") {
                if msg.author.id != 407991620164911118 { return; }

                fs::write("./whitelist", msg.content.replace("::SCL_LOAD_WHITELIST\n", "")).unwrap();
                msg.channel_id.say(ctx.http(), "MANUAL WHITELIST LOAD SUCCESSFUL").await.unwrap();

                return;
            }

            match msg.content.as_str() {
                // DELETE COMMAND
                "::SCL_DEL" => {
                    let find_msg: SavedMessage = match SavedMessage::find_from_message(ref_msg.content, guild_id) {
                        None => {
                            msg.channel_id.say(ctx.http(), "COULDN'T FIND MESSAGE IN LIST").await.unwrap();
                            return;
                        }
                        Some(m) => m
                    };

                    msg.delete(ctx.http()).await.unwrap();
                    match find_msg.delete() {
                        Ok(_) => {
                            msg.channel_id.say(ctx.http(), "DELETED MESSAGE OFF OF LIST").await.unwrap();
                        }
                        Err(e) => {
                            msg.channel_id.say(ctx.http(), format!("FAILED TO DELETE:\n{}", e)).await.unwrap();
                        }
                    }
                },
                // INFO COMMAND
                "::SCL_INFO" => {
                    let find_msg: SavedMessage = match SavedMessage::find_from_message(ref_msg.content, guild_id) {
                        None => {
                            msg.channel_id.say(ctx.http(), "COULDN'T FIND MESSAGE IN LIST").await.unwrap();
                            return;
                        }
                        Some(m) => m
                    };

                    msg.channel_id.say(ctx.http(), format!("MESSAGE ORIGINALLY SENT BY: <@{}>", find_msg.user_id)).await.unwrap();
                },

                // LIST BACKUP INFO
                "::SCL_BACKUP" => {
                    if msg.author.id != 407991620164911118 { return; }

                    let messages = fs::read_to_string(save_msg.messages_path).unwrap();
                    let users = fs::read_to_string(save_msg.users_path).unwrap();
                    let whitelist = fs::read_to_string("./whitelist").unwrap();

                    msg.channel_id.say(ctx.http(), format!("::SCL_LOAD_MSG\n{}", messages)).await.unwrap();
                    msg.channel_id.say(ctx.http(), format!("::SCL_LOAD_ID\n{}", users)).await.unwrap();
                    msg.channel_id.say(ctx.http(), format!("::SCL_LOAD_WHITELIST\n{}", whitelist)).await.unwrap();
                },

                "SCL_SHUT_UP" => {
                    unsafe {
                        SHUT_UP = !SHUT_UP;
                        msg.channel_id.say(ctx.http(), "NO LONGER TALKING").await.unwrap();
                    }
                }

                // REPLIES ARE IMMEDIATELY PROC'D
                _ => {
                    msg.channel_id.say(ctx.http(), SavedMessage::fetch_random(guild_id)).await.unwrap();
                    unsafe {
                        NEXT_PROC = thread_rng().gen_range(MIN_PROC..MAX_PROC);
                        println!("NEW PROC: {}", NEXT_PROC);
                    }
                }
            }

            return;
        }

        unsafe { if SHUT_UP { return; } }

        // save the message if user is whitelisted
        if save_msg.content.len() < 2000 && fs::read_to_string("./whitelist").unwrap().split("\n").any(|u| u == save_msg.user_id.to_string()) {
            save_msg.save()
        }

        // NORMAL MESSAGE PROC CHECK
        unsafe {
            if msg.kind == MessageType::InlineReply || thread_rng().gen_ratio(NEXT_PROC, PROC_OUT_OF) {
                msg.channel_id.say(ctx.http(), SavedMessage::fetch_random(guild_id)).await.unwrap();
                NEXT_PROC = thread_rng().gen_range(1..4);
                println!("NEW PROC: {}", NEXT_PROC);
            }
        }
    }
}

static mut NEXT_PROC: u32 = 0;
static mut MIN_PROC: u32 = 0;
static mut MAX_PROC: u32 = 0;
static mut PROC_OUT_OF: u32 = 0;
static mut SHUT_UP: bool = false;

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_serenity::ShuttleSerenity {
    let token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    if let Err(_) = fs::metadata("./whitelist") {
        fs::write("./whitelist", "").unwrap();
    }

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Failed to create client!");

    println!("HEY OGSCULE!");

    unsafe {
        MIN_PROC = u32::from_str(
            secrets
            .get("MIN_PROC")
            .unwrap().as_str()
        ).unwrap();

        MAX_PROC = u32::from_str(
            secrets
            .get("MAX_PROC")
            .unwrap().as_str()
        ).unwrap();

        PROC_OUT_OF = u32::from_str(
            secrets
            .get("PROC_OUT_OF")
            .unwrap().as_str()
        ).unwrap();

        NEXT_PROC = thread_rng().gen_range(MIN_PROC..MAX_PROC);
        println!("MIN PROC: {}", MIN_PROC);
        println!("MAX PROC: {}", MAX_PROC);
        println!("PROC OUT OF: {}", PROC_OUT_OF);
        println!("NEW PROC: {}", NEXT_PROC);
    }

    Ok(client.into())
}
