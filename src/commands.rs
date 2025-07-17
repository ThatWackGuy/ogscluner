use poise::serenity_prelude::*;
use crate::scluner_backup::{SclunerBackup, SclunerBackupCompat};
use crate::{Context, DataContext, Error};

fn fix_say_result<U>(faulty: Result<U>) -> Result<(), Error> {
    match faulty {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into())
    }
}

pub fn ctx_prefix<'a>(ctx: &'a Context<'a>) -> &'a DataContext {
    match ctx {
        Context::Application(_) => panic!("That shouldn't happen!"),
        Context::Prefix(c) => c as &DataContext
    }
}

// CHECKS
pub async fn dev_check(ctx: Context<'_>) -> Result<bool, Error> {
    Ok(ctx_prefix(&ctx).msg.author.id == 407991620164911118)
}

pub async fn mod_check(ctx: Context<'_>) -> Result<bool, Error> {
    let data = ctx.data().lock().await;
    let msg = ctx_prefix(&ctx).msg;
    Ok(msg.author.id == 407991620164911118 || ( data.modlist.contains(&msg.author.id) && !data.blacklist.contains(&msg.author.id) ))
}

pub async fn user_check(ctx: Context<'_>) -> Result<bool, Error> {
    let msg = ctx_prefix(&ctx).msg;
    Ok(!ctx.data().lock().await.blacklist.contains(&msg.author.id))
}

/// USER COMMAND
/// Deletes all memories with the given content
#[poise::command(prefix_command, guild_only, check="user_check")]
pub async fn delete_content(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().lock().await;
    let msg = ctx_prefix(&ctx).msg;
    let msg = match &msg.referenced_message {
        None => return fix_say_result(ctx.channel_id().say(ctx.http(), "PLEASE REPLY TO MESSAGE TO DELETE").await),
        Some(m) => m
    };

    msg.delete(ctx.http()).await?;
    let guild = data.guilds.get_mut(&ctx.guild_id().unwrap()).unwrap();

    guild.delete_message_content(msg.content.clone());

    fix_say_result(ctx.channel_id().say(ctx.http(), "DELETED ALL MESSAGES WITH CONTENT").await)
}

/// USER COMMAND
/// Fetches the information of the posted memory
#[poise::command(prefix_command, guild_only, check="user_check")]
pub async fn info_content(ctx: Context<'_>) -> Result<(), Error> {
    let msg = ctx_prefix(&ctx).msg;

    let m = match &msg.referenced_message {
        None => return fix_say_result(ctx.channel_id().say(ctx.http(), "PLEASE REPLY TO A SCLUNER MESSAGE TO USE AS CONTENT").await),
        Some(m) => {
            m
        }
    };

    let mut data = ctx.data().lock().await;
    let guild = data.guilds.get_mut(&ctx.guild_id().unwrap()).unwrap();
    let mut fetched_info = "MESSAGE ORIGINALLY SENT BY USERS:\n".to_string();

    for fetched in guild.fetch_from_content(m.content.clone()) {
        fetched_info += format!("<@{}>\n", fetched.user_id).as_str();
    }

    fix_say_result(ctx.channel_id().say(ctx.http(), fetched_info).await)
}

/// USER COMMAND
/// Shows the proc variables
#[poise::command(prefix_command, guild_only, check="user_check")]
pub async fn info_proc(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().lock().await;
    let guild = data.guilds.get_mut(&ctx.guild_id().unwrap()).unwrap();

    let info = format!("MIN_PROC:{}\nMAX_PROC:{}\nPROC_OUT_OF:{}\nCHANCE OF RANDOM REPLY: [{}..{}] out of {} tries",
        guild.min_proc,
        guild.max_proc,
        guild.proc_out_of,
        guild.min_proc, guild.max_proc, guild.proc_out_of,
    );

    fix_say_result(ctx.channel_id().say(ctx.http(), info).await)
}

/// USER COMMAND
/// Shows the proc variables
#[poise::command(prefix_command, guild_only, check="user_check")]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().lock().await;

    let running_time = data.startup_instant.elapsed().as_secs() / 3600;
    let backup_time = data.backup_instant.elapsed().as_secs() / 3600;
    let guilds_len = data.guilds.len();

    let guild = data.guilds.get_mut(&ctx.guild_id().unwrap()).unwrap();

    let info = format!("SCLUNER v{}\nRUNNING FOR: {}h\nTIME SINCE BACKUP: {}h\nON {} GUILDS\nSTORING {} MESSAGES ON CURRENT ONE",
        env!("CARGO_PKG_VERSION"),
        running_time,
        backup_time,
        guilds_len,
        guild.messages.len()
    );

    fix_say_result(ctx.channel_id().say(ctx.http(), info).await)
}

/// USER COMMAND
/// Registers or unregisters the user
#[poise::command(prefix_command, guild_only, check="user_check")]
pub async fn whitelist(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().lock().await;
    let user_id = ctx.author().id;

    if data.whitelist.contains(&user_id) {
        data.whitelist.retain(|u| *u != user_id);
        fix_say_result(ctx.channel_id().say(ctx.http(), format!("REMOVED USER <@{}>", user_id)).await)
    }
    else {
        data.whitelist.push(user_id);
        fix_say_result(ctx.channel_id().say(ctx.http(), format!("ADDED USER <@{}>", user_id)).await)
    }
}

/// MODERATOR COMMAND
/// Deletes all memories by given user
#[poise::command(prefix_command, guild_only, check="mod_check")]
pub async fn delete_user(ctx: Context<'_>, user: User) -> Result<(), Error> {
    ctx.data().lock().await.guilds.get_mut(&ctx.guild_id().unwrap()).unwrap().delete_message_sender(user.id);

    fix_say_result(ctx.channel_id().say(ctx.http(), format!("DELETED ALL MESSAGES SENT BY <@{}>", user.id)).await)
}

/// MODERATOR COMMAND
/// Sets the proc variables
#[poise::command(prefix_command, guild_only, check="mod_check")]
pub async fn proc(ctx: Context<'_>, min: u32, max: u32, out_of: u32) -> Result<(), Error> {
    let mut data = ctx.data().lock().await;
    let guild = data.guilds.get_mut(&ctx.guild_id().unwrap()).unwrap();

    guild.min_proc = min;
    guild.max_proc = max;
    guild.proc_out_of = out_of;

    fix_say_result(ctx.channel_id().say(ctx.http(), "SUCCESSFULLY SET PROC VARS").await)
}

/// MODERATOR COMMAND
/// mutes or unmutes the bot
#[poise::command(prefix_command, guild_only, check="mod_check")]
pub async fn sleep(ctx: Context<'_>) -> Result<(), Error> {
    let mut data = ctx.data().lock().await;
    let guild = data.guilds.get_mut(&ctx.guild_id().unwrap()).unwrap();

    guild.asleep = !guild.asleep;

    match guild.asleep {
        true => fix_say_result(ctx.channel_id().say(ctx.http(), "A mimir").await),
        false => fix_say_result(ctx.channel_id().say(ctx.http(), "Good morning!").await)
    }
}

/// DEV COMMAND
/// Adds a moderator
#[poise::command(prefix_command, guild_only, check="dev_check")]
pub async fn moderator(ctx: Context<'_>, user: User) -> Result<(), Error> {
    let mut data = ctx.data().lock().await;
    if data.modlist.contains(&user.id) {
        data.modlist.retain(|u| *u != user.id);
        fix_say_result(ctx.channel_id().say(ctx.http(), format!("REMOVED MODERATOR <@{}>", user.id)).await)
    }
    else {
        data.modlist.push(user.id);
        fix_say_result(ctx.channel_id().say(ctx.http(), format!("ADDED MODERATOR <@{}>", user.id)).await)
    }
}

/// DEV COMMAND
/// Forces a backup
#[poise::command(prefix_command, guild_only, check="dev_check")]
pub async fn backup_send(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data().lock().await;
    data.save_backup(ctx.serenity_context()).await;
    fix_say_result(ctx.channel_id().say(ctx.http(), "SUCCESSFULLY SENT BACKUP TO CHANNEL").await)
}

/// DEV COMMAND
/// Loads given backup
#[poise::command(prefix_command, guild_only, check="dev_check")]
pub async fn backup_load(ctx: Context<'_>, file: Attachment) -> Result<(), Error> {
    let backup_bytes = match file.download().await {
        Ok(f) => f,
        Err(e) => return fix_say_result(ctx.channel_id().say(ctx.http(), format!("FILE COULDN'T BE DOWNLOADED: {}", e)).await)
    };


    let backup = match ciborium::from_reader::<SclunerBackup, &[u8]>(&backup_bytes) {
        Ok(b) => b,
        Err(e) => return fix_say_result(ctx.channel_id().say(ctx.http(), format!("FILE COULDN'T BE DESERIALIZED: {}", e)).await)
    };

    ctx.data().lock().await.load_backup(backup);

    fix_say_result(ctx.channel_id().say(ctx.http(), "SUCCESSFULLY LOADED BACKUP").await)
}

#[poise::command(prefix_command, guild_only, check="dev_check")]
pub async fn backup_load_compat(ctx: Context<'_>, file: Attachment) -> Result<(), Error> {
    let backup_bytes = match file.download().await {
        Ok(f) => f,
        Err(e) => return fix_say_result(ctx.channel_id().say(ctx.http(), format!("FILE COULDN'T BE DOWNLOADED: {}", e)).await)
    };


    let backup = match ciborium::from_reader::<SclunerBackupCompat, &[u8]>(&backup_bytes) {
        Ok(b) => b,
        Err(e) => return fix_say_result(ctx.channel_id().say(ctx.http(), format!("FILE COULDN'T BE DESERIALIZED: {}", e)).await)
    };

    ctx.data().lock().await.load_backup(backup.modernise());

    fix_say_result(ctx.channel_id().say(ctx.http(), "SUCCESSFULLY COMPAT LOADED BACKUP").await)
}