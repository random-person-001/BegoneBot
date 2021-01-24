use rusqlite::Connection;
use crate::db::{MyDbContext, Settings, Action};
use serenity::prelude::Context;
use serenity::framework::standard::CommandResult;

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
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    match &choice[..] {
        "ban" => dbcontext.set_action(msg.guild_id.0, Action::Ban),
        "kick" => dbcontext.set_action(msg.guild_id.0, Action::Kick),
        "mute" => dbcontext.set_action(msg.guild_id.0, Action::Mute),
        default => {
            msg.channel_id.say(&ctx.http, "Sorry that wasn't recognized.  Recognized options are ban, kick, or mute").await?;
        },
    }
    msg.channel_id.say(&ctx.http, "Updated.").await?;
    Ok(())
}


// plan:

pub fn main() {
    let conn = Connection::open(":memory:").unwrap();
    let mut context = MyDbContext::new(&conn);
    context.add_guild(9);
    let s = context.fetch_settings(9);
    println!("{:?}", s);
    panic!["Yeet done"];
}
