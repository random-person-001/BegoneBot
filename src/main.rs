#![allow(unused)]
#![allow(non_snake_case)]
use crate::db::MyDbContext;
use serenity::model::prelude::*;
use sqlx::Result;

use serenity::{
    async_trait,
    client::bridge::gateway::{GatewayIntents, ShardId, ShardManager},
    framework::standard::{
        buckets::{LimitedFor, RevertBucket},
        help_commands,
        macros::{command, group, help, hook},
        Args, CommandGroup, CommandOptions, CommandResult, DispatchError, HelpOptions, Reason,
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
use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Write,
    sync::Arc,
};

use serenity::prelude::*;
use tokio::sync::Mutex;
mod admin_commands;
mod autopanic;
mod blob_blacklist_conversions;
mod commands;
mod db;

use crate::admin_commands::*;
use crate::autopanic::*;
use crate::commands::*;
use std::convert::TryInto;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

// A container type is created for inserting into the Client's `data`, which
// allows for data to be accessible across all events and framework commands, or
// anywhere else that has a copy of the `data` Arc.
struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

impl TypeMapKey for db::MyDbContext {
    type Value = db::MyDbContext;
}

impl TypeMapKey for autopanic::Gramma {
    type Value = autopanic::Gramma;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        let mut data = ctx.data.write().await;
        let mut dbcontext = data
            .get_mut::<MyDbContext>()
            .expect("Expected MyDbContext in TypeMap.");
        let id = &guild.id.0;
        if let Some(s) = dbcontext.fetch_settings(id).await {
            println!("Found guild {} settings", id);
            dbcontext.cache.insert(*id, s);
        } else {
            println!("Creating a new settings row for guild {}", id);
            dbcontext.add_guild(id).await; // also adds to cache
            //greet_new_guild(&ctx, &guild).await;
        };
        set_status(&ctx).await;
    }

    async fn channel_pins_update(&self, ctx: Context, _pins: ChannelPinsUpdateEvent) {
        println!("yeet doing a garbage run");
        garbage_collect(&ctx);
        println!("done");
    }

    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, mut new_member: Member) {
        println!("new member joined {}: {}", guild_id, new_member.user.name);
        {
            let mut data = ctx.data.write().await;
            let mut grammy = data
                .get_mut::<autopanic::Gramma>()
                .expect("Expected your momma in TypeMap.");
            let mut mom = grammy.get(&guild_id.0);
            mom.recent_users.insert(
                new_member
                    .joined_at
                    .unwrap()
                    .timestamp_millis()
                    .try_into()
                    .unwrap(),
                new_member.user.id.0,
            );
        }
        check_against_joins(&ctx, guild_id.0).await;
        check_against_blacklist(&ctx, new_member, guild_id.0).await;
    }

    async fn message(&self, ctx: Context, new_message: Message) {
        /*
        if new_message.content.len() > 20_usize {
            println!("Message! {}...", &new_message.content[..19]);
        } else {
            println!("Message! {}", &new_message.content);
        }*/
        // we use the message timestamp instead of time::now because of potential lag of events
        let timestamp: u64 = new_message.timestamp.timestamp_millis().try_into().unwrap();
        let guild = new_message.guild_id.unwrap().0;
        let author = new_message.author.id.0;
        let mut data = ctx.data.write().await;
        let mut grammy = data
            .get_mut::<autopanic::Gramma>()
            .expect("Expected your momma in TypeMap.");
        let mut mom = grammy.get(&guild);

        if !new_message.mentions.is_empty() {
            mom.userpings
                .insert(timestamp, (new_message.mentions.len(), author));
        }
        if !new_message.mention_roles.is_empty() {
            mom.rollpings
                .insert(timestamp, (new_message.mentions.len(), author));
        }
        if !new_message.mention_roles.is_empty() || !new_message.mentions.is_empty() {
            autopanic::check_against_pings(&ctx, mom, guild).await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

pub async fn better_default_channel(guild: &Guild, uid: UserId) -> Option<Vec<&GuildChannel>> {
    let member = guild.members.get(&uid)?;
    let mut out = vec![];

    for channel in guild.channels.values() {
        if channel.kind == ChannelType::Text
            && guild
                .user_permissions_in(channel, member)
                .ok()?
                .send_messages()
            && guild
                .user_permissions_in(channel, member)
                .ok()?
                .read_messages()
        {
            let x = guild.user_permissions_in(channel, member).expect("goo");
            //return Some(channel);
            dbg!(x);
            println!("{:?}", x.bits);
            println!("{}", channel.name);
            out.push(channel);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

async fn greet_new_guild(ctx: &Context, guild: &Guild) {
    println!("h");
    if let Some(channelvec) = better_default_channel(guild, UserId(802019556801511424_u64)).await {
        println!("i");
        for channel in channelvec {
            println!("{}", channel.name);
            let res = channel.say(&ctx, "
            Thanks for adding me to the server! Here's some next steps:\n
Configure who can run most commands (like turning on or off panic mode): run `bb-settings set roll_that_can_panic Staff` for example (if you have a roll called Staff)\n
I recommend that you set up a log channel for me to talk in (and set it like `bb-settings set logs #mychannel` but replace mychannel with the actual one) \n
Also probs tell me a roll for me to ping when I automatically detect a raid and go into panic mode (`bb-settings set notify raidresponders` - replacing raidresponders with that roll)\n
Reviewing default settings is recommended - `bb-settings` and adjust them as you wish. `bb-help` shows all my commands.\n
If you find yourself needing support, there's a support server invite in `bb-about`\
            ").await;
            if res.is_ok() {
                return;
            }
        }
    } else {
        println!(
            "hey i wanted to greet {} {} but they wont let everyone talk",
            guild.name, guild.id.0
        );
    }
}

async fn set_status(ctx: &Context) {
    ctx.shard.set_status(OnlineStatus::DoNotDisturb);
    let s = format!("to {} guilds | bb-help", ctx.cache.guild_count().await);
    ctx.shard.set_activity(Some(Activity::listening(&*s)));
}

#[group]
#[commands(panic, uinfo, forceban, help, delete)]
struct General;

#[group]
#[commands(about, ping, die, update, free, git_push, garbage, foo)] // status)]
struct Meta;

#[group]
// Sets multiple prefixes for a group.
// This requires us to call commands in this group
// via `~emoji` (or `~em`) instead of just `~`.
#[prefixes("settings", "s")]
// Set a description to appear if a user wants to display a single group
// e.g. via help using the group-name or one of its prefixes.
// Summary only appears when listing multiple groups.
// Sets a command that will be executed if only a group-prefix was passed.
#[default_command(show)]
#[commands(reset, set)]
struct Settings;

#[group]
#[prefixes("blacklist", "bl")]
#[default_command(blacklist_show)]
#[commands(remove, add)]
struct Blacklist;

#[hook] // this appears not to work
async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
    println!(
        "Got command '{}' by user '{}'",
        command_name, msg.author.name
    );
    true // if `before` returns false, command processing doesn't happen.
}

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
    match command_result {
        Ok(()) => println!("Processed command '{}'", command_name),
        Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
    }
}

#[hook]
async fn unknown_command(_ctx: &Context, _msg: &Message, unknown_command_name: &str) {
    println!("Could not find command named '{}'", unknown_command_name);
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    if let DispatchError::Ratelimited(info) = error {
        // We notify them only once.
        if info.is_first_try {
            let _ = msg
                .channel_id
                .say(
                    &ctx.http,
                    &format!("Try this again in {} seconds.", info.as_secs()),
                )
                .await;
        }
    }
}

#[tokio::main]
async fn main() {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    println!("{:?}", since_the_epoch);
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let bot_id = match http.get_current_application_info().await {
        Ok(_) => match http.get_current_user().await {
            Ok(bot_id) => bot_id.id,
            Err(why) => panic!("Could not access the bot id: {:?}", why),
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| {
            c.with_whitespace(true)
                .on_mention(Some(bot_id))
                .prefix("bb-")
                .case_insensitivity(true)
                .allow_dm(false)
        })
        .unrecognised_command(unknown_command)
        // Set a function that's called whenever a command's execution didn't complete for one
        // reason or another. For example, when a user has exceeded a rate-limit or a command
        // can only be performed by the bot owner.
        .on_dispatch_error(dispatch_error)
        // The `#[group]` macro generates `static` instances of the options set for the group.
        // They're made in the pattern: `#name_GROUP` for the group instance and `#name_GROUP_OPTIONS`.
        // #name is turned all uppercase
        .group(&GENERAL_GROUP)
        .group(&BLACKLIST_GROUP)
        .group(&SETTINGS_GROUP)
        .group(&META_GROUP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .intents(
            GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::privileged(),
        )
        .await
        .expect("Err creating client");

    {
        let conn = sqlx::SqlitePool::connect("db.sqlite").await;
        let mut data = client.data.write().await;
        data.insert::<db::MyDbContext>(MyDbContext::new(conn.unwrap()));
        data.insert::<autopanic::Gramma>(autopanic::Gramma::new());
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}



async fn garbage_collect(ctx: &Context) {
    let now = autopanic::time_now();
    let mut data = ctx.data.write().await;

    let settings_map = &data
        .get::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.").cache.clone();


    let mut grammy = data
        .get_mut::<autopanic::Gramma>()
        .expect("Expected your momma in TypeMap.");

    // iterate over gramma to get your mom for each guild
    // each your mom will have a settings attached, as well as memory of joins and whatnot
    // make and save a new list of just the joins that are currently relevant and discard the previous

    for (k, v) in grammy.guild_mamas.iter_mut() {
        if let Some(settings) = settings_map.get(&k) {

            let max_age = settings.time;  // duration we keep join records, in seconds
            let mut new_recent_users: HashMap<u64, u64> = HashMap::new();
            for (timestamp, user) in v.recent_users.iter_mut() {  // timestamp joined , userid
                if !autopanic::time_is_past(*timestamp, max_age as u64) {
                    new_recent_users.insert(*timestamp, *user);
                }
            }
            v.recent_users = new_recent_users;


            let max_age = settings.mentiontime;
            let mut new_userpings: HashMap<u64, (usize, u64)> = HashMap::new();
            for (timestamp, user) in v.userpings.iter() {
                if !autopanic::time_is_past(*timestamp, max_age as u64) {
                    new_userpings.insert(*timestamp, *user);
                }
            }
            v.userpings = new_userpings;



            let mut new_rollpings: HashMap<u64, (usize, u64)> = HashMap::new();
            for (timestamp, user) in v.rollpings.iter() {
                if !autopanic::time_is_past(*timestamp, max_age as u64) {
                    new_rollpings.insert(*timestamp, *user);
                }
            }
            v.rollpings = new_rollpings;

        }

    }

}
