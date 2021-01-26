use crate::db::{MyDbContext, Settings, Action};
use serenity::prelude::Context;
use serenity::framework::standard::CommandResult;
use std::convert::TryInto;

use serenity::{
    async_trait,
    client::bridge::gateway::{ShardId, ShardManager},
    framework::standard::{
        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{check, command, group, help, hook},
        Args, CommandGroup, CommandOptions, DispatchError, HelpOptions, Reason,
        StandardFramework,
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
/*
? / help      display help
on/off        enable/disable auto panic
current       display current settings
users <#>     User joins needed to trigger AP
time <#>      time window to trigger AP
action        choose whether to ban, kick, or mute upon AP
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
    let choice =  args.clone().single::<String>().unwrap().to_lowercase();
    let mut data = ctx.data.write().await;
    let guild:u64 = match msg.guild_id {
        Some(id) => {
            println!("{:?}", id);
            println!("{:?}", id.0);
            id.0.try_into().unwrap()
        },
        None => 0,
    };
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    match &choice[..] {
        "ban" => dbcontext.set_action(guild, Action::Ban).await,
        "kick" => dbcontext.set_action(guild, Action::Kick).await,
        "mute" => dbcontext.set_action(guild, Action::Mute).await,
        default => {
            msg.channel_id.say(&ctx.http, "Sorry that wasn't recognized.  Recognized options are ban, kick, or mute").await?;
            false
        },
    };
    msg.channel_id.say(&ctx.http, "Updated.").await?;
    Ok(())
}


