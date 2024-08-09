use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::borrow::BorrowMut;
use std::num::ParseIntError;
use rand::prelude::*;
use rand::thread_rng;
use serenity::all::*;
use shuttle_runtime::{SecretStore, tokio};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct SclunerMessage {
    user_id: UserId,
    content: String
}

impl SclunerMessage {
    fn new(msg: &Message) -> Self {
        Self {
            user_id: msg.author.id.clone(),
            content: msg.content.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct SclunerGuild {
    guild_id: GuildId,
    messages: Vec<SclunerMessage>,
    shut_up: bool,

    min_proc: u32,
    max_proc: u32,
    proc_out_of: u32,

    proc: u32,
}

impl SclunerGuild {
    fn new(guild_id: GuildId) -> Self {
        Self {
            guild_id,
            messages: Vec::new(),
            shut_up: false,

            min_proc: 1,
            max_proc: 4,
            proc_out_of: 18,

            proc: thread_rng().gen_range(1..4)
        }
    }

    fn fetch_random(&mut self) -> Option<String> {
        let messages = &self.messages;
        match messages.choose(&mut thread_rng()) {
            None => None,
            Some(c) => Some(c.content.clone())
        }
    }

    fn fetch_from_content(&mut self, content: String) -> Vec<&SclunerMessage> {
        self.messages.iter().filter(|m| m.content == content).collect()
    }

    fn fetch_from_user(&mut self, user_id: UserId) -> Vec<&SclunerMessage> {
        self.messages.iter().filter(|m| m.user_id == user_id).collect()
    }

    fn delete_message_sender(&mut self, user_id: UserId) {
        self.messages.retain(|m| m.user_id != user_id);
    }

    fn delete_message_content(&mut self, content: &String) {
        self.messages.retain(|m| m.content != *content);
    }
}

struct SclunerHandler {
    instance: Arc<Mutex<SclunerInstance>>
}

#[async_trait]
impl EventHandler for SclunerHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot { return; }

        let guild_id = match msg.guild_id {
            None => return,
            Some(id) => id
        };

        match msg.referenced_message {
            None => {},
            Some(ref m) => {
                let reply_cmd = msg.mentions_me(ctx.http()).await.unwrap();
                if reply_cmd {
                    if msg.content.starts_with("::SCL_DEL_USER ") {
                        if !self.permission_dev(msg.author.id) && !self.permission_mod(msg.author.id) { return; }

                        let user_id = match UserId::from_str(msg.content.replace("::SCL_DEL_USER ", "").as_str()) {
                            Ok(id) => id,
                            Err(_) => {
                                if let Err(e) = msg.channel_id.say(ctx.http(), "GIVEN USERID WASN'T RIGHT").await {
                                    eprintln!("FAILED TO SEND MESSAGE DELETE USER ERROR: {}", e);
                                };

                                return;
                            }
                        };

                        {
                            let mut instance = self.instance.lock().unwrap();
                            instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id)).delete_message_sender(user_id);
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), format!("DELETED ALL MESSAGES SENT BY <@{}>", user_id)).await {
                            eprintln!("FAILED TO SEND MESSAGE DELETE USER CONFIRM: {}", e);
                        }

                        return;
                    }
                    else if msg.content.starts_with("::SCL_INFO_USER ") {
                        if !self.permission_dev(msg.author.id) && !self.permission_mod(msg.author.id) { return; }

                        let user_id = match UserId::from_str(msg.content.replace("::SCL_INFO_USER ", "").as_str()) {
                            Ok(id) => id,
                            Err(_) => {
                                if let Err(e) = msg.channel_id.say(ctx.http(), "GIVEN USERID WASN'T RIGHT").await {
                                    eprintln!("FAILED TO SEND MESSAGE DELETE USER ERROR: {}", e);
                                };

                                return;
                            }
                        };

                        let mut fetched_info = "MESSAGES SENT BY GIVEN USER:\n".to_string();

                        {
                            let mut instance = self.instance.lock().unwrap();
                            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));
                            for fetched in guild.fetch_from_user(user_id) {
                                fetched_info = fetched_info + format!("{}\n", fetched.content).as_str();
                            }
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), fetched_info).await {
                            eprintln!("FAILED TO SEND MESSAGE INFO FROM USER: {}", e);
                        }

                        return;
                    }
                    else if msg.content.starts_with("::SCL_PROC ") {
                        if !self.permission_dev(msg.author.id) && !self.permission_mod(msg.author.id) { return; }

                        let procs: Vec<Result<u32, ParseIntError>> = msg.content.replace("::SCL_PROC ", "").split(" ").map(|p| u32::from_str(p)).collect();
                        if procs.len() != 3 || procs.iter().any(|p| p.is_err()) {
                            if let Err(e) = msg.channel_id.say(ctx.http(), "COMMAND IS ::SCL_PROC MIN MAX OUT_OF\nALL 32 BIT NUMBERS").await {
                                eprintln!("FAILED TO PROC REASSIGN: {}", e);
                            }
                        }

                        let proc_nums: Vec<u32> = procs.iter().map(|p| p.clone().unwrap()).collect();

                        let min = proc_nums.get(0).unwrap();
                        let max = proc_nums.get(1).unwrap();
                        let out_of = proc_nums.get(2).unwrap();

                        {
                            let mut instance = self.instance.lock().unwrap();
                            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));

                            guild.min_proc = *min;
                            guild.max_proc = *max;
                            guild.proc_out_of = *out_of;
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), "SUCCESSFULLY SET PROC VARS").await {
                            eprintln!("FAILED TO PROC REASSIGN CONFIRM: {}", e);
                        }

                        return;
                    }
                    else if msg.content.starts_with("::SCL_MOD ") {
                        if !self.permission_dev(msg.author.id) { return; }

                        let user_id = match UserId::from_str(msg.content.replace("::SCL_MOD ", "").as_str()) {
                            Ok(id) => id,
                            Err(_) => {
                                if let Err(e) = msg.channel_id.say(ctx.http(), "GIVEN USERID WASN'T RIGHT").await {
                                    eprintln!("FAILED TO SEND MODERATOR SET ERROR: {}", e);
                                };

                                return;
                            }
                        };

                        {
                            let mut instance = self.instance.lock().unwrap();
                            instance.modlist.push(user_id)
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), format!("ADDED MODERATOR <@{}>", user_id)).await {
                            eprintln!("FAILED TO SEND MOD SET CONFIRMATION: {}", e);
                        }

                        return;
                    }
                }

                match msg.content.as_str() {
                    "::SCL_DEL_CONTENT" => {
                        if !reply_cmd || !self.permission(msg.author.id) { return; }

                        if let Err(e) = m.delete(ctx.http()).await {
                            eprintln!("FAILED TO DELETE REQUESTED DELETE SCLUNER MESSAGE: {}", e);
                        }

                        {
                            let mut instance = self.instance.lock().unwrap();
                            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));

                            guild.delete_message_content(&msg.content);
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), "DELETED ALL MESSAGES WITH CONTENT").await {
                            eprintln!("FAILED TO SEND MESSAGE DELETE CONTENT CONFIRM: {}", e);
                        }

                        return;
                    },
                    "::SCL_INFO_CONTENT" => {
                        if !reply_cmd || !self.permission(msg.author.id) { return; }

                        let mut fetched_info = "MESSAGE ORIGINALLY SENT BY USERS:\n".to_string();

                        {
                            let mut instance = self.instance.lock().unwrap();
                            for fetched in instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id)).fetch_from_content(m.content.clone()) {
                                fetched_info = fetched_info + format!("<@{}>\n", fetched.user_id).as_str();
                            }
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), fetched_info).await {
                            eprintln!("FAILED TO SEND MESSAGE INFO FROM CONTENT: {}", e);
                        }

                        return;
                    },
                    "::SCL_PROC" => {
                        if !reply_cmd || !self.permission(msg.author.id) { return; }

                        let info = {
                            let mut instance = self.instance.lock().unwrap();
                            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));

                            format!("MIN_PROC: {}\nMAX_PROC: {}\nPROC_OUT_OF:{}\nCHANCE OF SENDING REPLY PER MESSAGE: [{}..{}] / {}",
                                    guild.min_proc,
                                    guild.max_proc,
                                    guild.proc_out_of,

                                    guild.min_proc,
                                    guild.max_proc,
                                    guild.proc_out_of,
                            )
                        };

                        if let Err(e) = msg.channel_id.say(ctx.http(), info).await {
                            eprintln!("FAILED TO PROC INFO: {}", e);
                        }

                        return;
                    },

                    "::SCL_SHUT_UP" => {
                        if !reply_cmd || (!self.permission_dev(msg.author.id) && !self.permission_mod(msg.author.id)) { return; }

                        let shut = {
                            let mut instance = self.instance.lock().unwrap();
                            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));
                            guild.shut_up = !guild.shut_up;

                            guild.shut_up
                        };

                        if let Err(e) = msg.channel_id.say(ctx.http(), format!("SCLUNER WILL NOW SHUT UP: {}", shut)).await {
                            eprintln!("FAILED TO SHUT UP CONFIRMATION: {}", e);
                        }

                        return;
                    },

                    "::SCL_BACKUP_SEND" => {
                        if !reply_cmd || !self.permission_dev(msg.author.id) { return; }

                        let backup;
                        {
                            let instance = self.instance.lock().unwrap();
                            backup = SclunerBackup::new(&instance.guilds, &instance.whitelist, &instance.blacklist, &instance.modlist);
                        }

                        backup.save(&ctx).await;

                        if let Err(e) = msg.channel_id.say(ctx.http(), "SUCCESSFULLY SENT BACKUP TO CHANNEL").await {
                            eprintln!("FAILED TO SEND BACKUP CONFIRMATION: {}", e);
                        }

                        return;
                    },
                    "::SCL_BACKUP_LOAD" => {
                        if !reply_cmd || !self.permission_dev(msg.author.id) { return; }

                        let file_download = match msg.attachments.get(0) {
                            None => {
                                if let Err(e) = msg.channel_id.say(ctx.http(), "FILE COULDN'T BE FOUND").await {
                                    eprintln!("FAILED TO SEND LOAD FAILURE: {}", e);
                                }

                                return;
                            }
                            Some(a) => a
                        }.download().await;

                        let backup = match file_download {
                            Ok(f) => {
                                match serde_cbor::from_slice::<SclunerBackup>(f.as_slice()) {
                                    Ok(b) => b,
                                    Err(e) => {
                                        if let Err(e) = msg.channel_id.say(ctx.http(), format!("FILE COULDN'T BE DESERIALIZED: {}", e)).await {
                                            eprintln!("FAILED TO SEND LOAD FAILURE: {}", e);
                                        }

                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                if let Err(e) = msg.channel_id.say(ctx.http(), format!("FILE COULDN'T BE DOWNLOADED: {}", e)).await {
                                    eprintln!("FAILED TO SEND LOAD FAILURE: {}", e);
                                }

                                return;
                            }
                        };

                        {
                            let mut instance = self.instance.lock().unwrap();
                            instance.load_backup(backup);
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), "SUCCESSFULLY LOADED BACKUP").await {
                            eprintln!("FAILED TO SEND LOAD CONFIRMATION: {}", e);
                        }

                        return;
                    },

                    _ => {
                        let rnd_msg;
                        {
                            let mut instance = self.instance.lock().unwrap();
                            rnd_msg = match instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id)).fetch_random() {
                                None => {
                                    eprintln!("FAILED TO FETCH ALWAYS PROC RANDOM RESPONSE! NOTHING IN MESSAGES");
                                    return;
                                }
                                Some(m) => m
                            };
                        }

                        if let Err(e) = msg.channel_id.say(ctx.http(), rnd_msg).await {
                            eprintln!("FAILED TO SEND RANDOM RESPONSE: {}", e);
                        }
                    }
                };
            }
        };

        {
            let mut instance = self.instance.lock().unwrap();
            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));

            if guild.shut_up { return; }
        }

        // scluner mention is special as it adds you to the whitelist
        if msg.mentions_me(ctx.http()).await.unwrap() {
            if !self.permission(msg.author.id) { return; }

            println!("ADDING {} TO THE WHITELIST..", msg.author.name);

            let mut instance = self.instance.lock().unwrap();
            let whitelist: &mut Vec<UserId> = instance.whitelist.borrow_mut();
            whitelist.push(msg.author.id.clone());

            return;
        }

        let mut send_random;
        let random_msg;
        {
            let mut instance = self.instance.lock().unwrap();
            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));

            send_random = thread_rng().gen_ratio(guild.proc, guild.proc_out_of);

            if send_random {
                guild.proc = thread_rng().gen_range(guild.min_proc..guild.max_proc);

                random_msg = match guild.fetch_random() {
                    None => {
                        eprintln!("FAILED TO FETCH RANDOM PROC RESPONSE! NOTHING IN MESSAGES");

                        send_random = false;
                        "".to_string()
                    }
                    Some(m) => m
                };
            }
            else {
                random_msg = "".to_string()
            }
        }

        if send_random {
            if let Err(e) = msg.channel_id.say(ctx.http(), random_msg).await {
                eprintln!("FAILED TO SEND RANDOM RESPONSE: {}", e);
            }
        }

        if self.permission(msg.author.id) && msg.content.len() > 0 && msg.mentions.len() == 0 && self.whitelisted(msg.author.id) {
            let mut instance = self.instance.lock().unwrap();
            let guild = instance.guilds.entry(guild_id).or_insert(SclunerGuild::new(guild_id));

            guild.messages.push(SclunerMessage::new(&msg));

            // 1k message limit
            if guild.messages.len() > 522 {
                guild.messages.remove(0);
            }
        }
    }
}

impl SclunerHandler {
    fn new() -> Self {
        Self {
            instance: Arc::new(Mutex::new(SclunerInstance::new()))
        }
    }

    fn permission_mod(&self, user_id: UserId) -> bool {
        let instance = self.instance.lock().unwrap();
        !instance.blacklist.contains(&user_id) && instance.modlist.contains(&user_id)
    }

    fn permission_dev(&self, user_id: UserId) -> bool {
        user_id == 407991620164911118
    }

    fn permission(&self, user_id: UserId) -> bool {
        let instance = self.instance.lock().unwrap();
        !instance.blacklist.contains(&user_id)
    }

    fn whitelisted(&self, user_id: UserId) -> bool {
        let instance = self.instance.lock().unwrap();
        instance.whitelist.contains(&user_id)
    }
}

struct SclunerInstance {
    backup_timestamp: Instant,
    backup_timer: Duration,
    guilds: HashMap<GuildId, SclunerGuild>,
    whitelist: Vec<UserId>,
    blacklist: Vec<UserId>,
    modlist: Vec<UserId>
}

impl SclunerInstance {
    fn new() -> Self {
        Self {
            backup_timestamp: Instant::now(),
            backup_timer: Duration::new(600, 0),
            guilds: HashMap::new(),
            whitelist: Vec::new(),
            blacklist: Vec::new(),
            modlist: Vec::new()
        }
    }

    fn load_backup(&mut self, load: SclunerBackup) {
        let guilds = load.guilds_keys.into_iter().zip(load.guilds_values.into_iter()).collect();

        self.backup_timestamp = Instant::now();
        self.backup_timer = Duration::new(600, 0);
        self.guilds = guilds;
        self.whitelist = load.whitelist;
        self.blacklist = load.blacklist;
        self.modlist = load.modlist;
    }
}


#[derive(Serialize, Deserialize)]
struct SclunerBackup {
    guilds_keys: Vec<GuildId>,
    guilds_values: Vec<SclunerGuild>,
    whitelist: Vec<UserId>,
    blacklist: Vec<UserId>,
    modlist: Vec<UserId>
}

impl SclunerBackup {
    fn new(guilds: &HashMap<GuildId, SclunerGuild>, whitelist: &Vec<UserId>, blacklist: &Vec<UserId>, modlist: &Vec<UserId>) -> Self {
        Self {
            guilds_keys: guilds.keys().cloned().collect(),
            guilds_values: guilds.values().cloned().collect(),
            whitelist: whitelist.clone(),
            blacklist: blacklist.clone(),
            modlist: modlist.clone(),
        }
    }

    async fn save(&self, ctx: &prelude::Context) {
        let serialised = match serde_cbor::to_vec(self) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("FAILED TO BACKUP : SERIALISATION UNSUCCESSFUL:{}", e);
                return;
            }
        };

        // unchecked byte-string conversion
        unsafe {
            if let Err(e) = fs::write("./backup", String::from_utf8_unchecked(serialised)) {
                eprintln!("FAILED TO BACKUP : WRITING UNSUCCESSFUL:{}", e);
                return;
            }
        }

        let backup = tokio::fs::OpenOptions::new()
            .read(true)
            .open("./backup")
            .await
            .unwrap();

        // private channel
        if let Err(e) = ChannelId::from(970308154401378356).send_files(ctx.http(), vec![CreateAttachment::file(&backup, "backup.txt").await.unwrap()], Default::default()).await {
            eprintln!("FAILED TO BACKUP : FILES COULDN'T BE SENT:{}", e);
            return;
        }

        println!("BACKUP SUCCESSFUL!");
    }
}

#[shuttle_runtime::main]
async fn main(#[shuttle_runtime::Secrets] secrets: SecretStore) -> shuttle_serenity::ShuttleSerenity {
    let token = secrets.get("DISCORD_TOKEN").expect("'DISCORD_TOKEN' was not found");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let client = Client::builder(&token, intents)
        .event_handler_arc(Arc::new(SclunerHandler::new()))
        .await
        .expect("Failed to create client!");

    println!("HEY OGSCULE!");

    Ok(client.into())
}
