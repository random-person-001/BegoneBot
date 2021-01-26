use crate::db::{Action, MyDbContext, Settings};
use serenity::framework::standard::CommandResult;
use serenity::prelude::Context;
use std::convert::TryInto;

use serenity::{
    async_trait,
    client::bridge::gateway::{ShardId, ShardManager},
    framework::standard::{
        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{check, command, group, help, hook},
        Args, CommandGroup, CommandOptions, DispatchError, HelpOptions, Reason, StandardFramework,
    },
    http::Http,
    model::{
        channel::{Channel, Message},
        gateway::Ready,
        guild::Guild,
        id::UserId,
        permissions::Permissions,
    },
    utils::{content_safe, ContentSafeOptions},
};
use serenity::model::channel::GuildChannel;
/*
? / help      display help
on/off        enable/disable auto panic
current       display current settings
users <#>     User joins needed to trigger AP
time <#>      time window to trigger AP
action        choose whether to ban, kick, or mute upon AP
now           turns on panic mode immediately
stop          turns off panic mode immediately
muteroll @r   sets @r to be the roll applied automatically to noobs during panic when action=mute
logs #chan    sets #chan to be where logs occur

**later**

blacklist     automatically apply `action` to any new join at any time if they have a bad -
 name         username
 avatar       pfp
 message      message content



some time after raid stops, turn panic mode off

 */

/*
fn on_user_join(&state, &user) {
    // check if autopanic is on
        // check if it should be turned off
        // else
            // punish the user

    // check if it should be turned on
        // punish the user
}*/

#[command]
async fn action(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let choice = args.clone().single::<String>().unwrap().to_lowercase();
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };
    match &choice[..] {
        "ban" => dbcontext.set_action(guild, Action::Ban).await,
        "kick" => dbcontext.set_action(guild, Action::Kick).await,
        "mute" => dbcontext.set_action(guild, Action::Mute).await,
        default => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Sorry that wasn't recognized.  Recognized options are ban, kick, or mute",
                )
                .await?;
            false
        }
    };
    msg.channel_id.say(&ctx.http, "Updated.").await?;
    Ok(())
}


#[command]
async fn reset(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let choice = args.clone().single::<String>().unwrap().to_lowercase();
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };

    if dbcontext.add_guild(guild).await {
        msg.channel_id
            .say(&ctx.http, "Successfully reset settings")
            .await?;
    } else {
        let s = "Problems were encountered while attempting to reset settings";
        msg.channel_id .say(   &ctx.http,s).await?;
    }
    Ok(())
}

#[command]
async fn current(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };

    let settings = dbcontext.fetch_settings(guild).await;
    let settings = settings.unwrap();

    let s = format!(
        r#"**Current antiraid settings**
    Automatic raid detection is currently {}
    Panic mode is triggered when {} users join in {} seconds.
    During panic mode, any member joining will be {}.
    {}.
    "#,
        if let 0 = settings.enabled {
            "**DISABLED!**"
        } else {
            "ENABLED."
        },
        settings.users,
        settings.time,
        match settings.action {
            Action::Ban => "banned",
            Action::Kick => "kicked",
            Action::Mute => "muted",
        },
        if let 0 = settings.logs {
            String::from("No logging channel is configured")
        } else {
            format!("Logs are posted in <#{}>", settings.logs)
        }
    );

    msg.channel_id.say(&ctx.http, s).await?;
    Ok(())
}

#[command]
async fn users(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let choice = match args.clone().single::<u8>() {
        Ok(n) => n,
        Err(why) => {
            msg.channel_id.say(&ctx.http, "Sorry, that wasn't recognized as a reasonable number of users to join in a given time.").await?;
            return Ok(());
        }
    };
    if choice <= 1 {
        msg.channel_id
            .say(
                &ctx.http,
                "That would kick anyone trying to join, so imma say nope to that chief.",
            )
            .await?;
        return Ok(());
    }
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };
    if dbcontext.set_users(guild, choice).await {
        msg.channel_id.say(&ctx.http, "Updated.").await?;
        Ok(())
    } else {
        msg.channel_id
            .say(&ctx.http, "Problems arose when trying to update settings.")
            .await?;
        Ok(())
    }
}


#[command]
async fn time(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };
    let choice = match args.clone().single::<u32>() {
        Ok(n) => n,
        Err(why) => {
            let s = "Sorry, that wasn't recognized as a reasonable amount of time in seconds (no fractions or decimals please)";
            msg.channel_id.say(&ctx.http, s).await?;
            return Ok(());
        }
    };
    if choice < 1 {
        let s = "That would kick anyone trying to join, so imma say nope to that chief.";
        msg.channel_id.say(&ctx.http, s).await?;
        return Ok(());
    }
    if dbcontext.set_time(guild, choice).await {
        msg.channel_id.say(&ctx.http, "Updated.").await?;
        Ok(())
    } else {
        msg.channel_id
            .say(&ctx.http, "Problems arose when trying to update settings.")
            .await?;
        Ok(())
    }
}

fn chan_str_to_id(s: &str) -> Option<u64> {
    // todo: this
    Some(0)
}

#[command]
async fn logs(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };
    let choice = args.clone().single::<String>().unwrap();
    let choice = match chan_str_to_id(&choice[..]) {
        Some(id) => id,
        None => {
            let s = "Bro that didn't look like a normal channel message :(";
            msg.channel_id .say(&ctx.http, s).await?;
            return Ok(())
        }
    };

    if dbcontext.set_logs(guild, choice).await {
        msg.channel_id.say(&ctx.http, "Updated.").await?;
        Ok(())
    } else {
        msg.channel_id
            .say(&ctx.http, "Problems arose when trying to update settings.")
            .await?;
        Ok(())
    }
}
