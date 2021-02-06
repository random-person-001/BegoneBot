#![allow(unused)]
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
mod autopanic;
mod commands;
mod db;
mod blob_blacklist_conversions;

use crate::autopanic::*;
use crate::commands::*;
use std::convert::TryInto;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};

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
        };
        set_status(&ctx).await;
    }

    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, new_member: Member) {
        println!("new member joined: {}", new_member.user.name);
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
    }

    async fn message(&self, ctx: Context, new_message: Message) {
        if new_message.content.len() > 20_usize {
            println!("Message! {}...", &new_message.content[..19]);
        } else {
            println!("Message! {}", &new_message.content);
        }
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

async fn set_status(ctx: &Context) {
    ctx.shard.set_status(OnlineStatus::DoNotDisturb);
    let s = format!("to {} guilds | bb-help", ctx.cache.guild_count().await);
    ctx.shard.set_activity(Some(Activity::listening(&*s)));
}

#[group]
#[commands(about, ping, invite, die, panic, uinfo, forceban)] // status)]
struct General;

#[group]
// Sets multiple prefixes for a group.
// This requires us to call commands in this group
// via `~emoji` (or `~em`) instead of just `~`.
#[prefixes("settings", "s")]
// Set a description to appear if a user wants to display a single group
// e.g. via help using the group-name or one of its prefixes.
#[description = "Adjust settings"]
// Summary only appears when listing multiple groups.
#[summary = "Adjust settings"]
// Sets a command that will be executed if only a group-prefix was passed.
#[default_command(show)]
#[commands(reset, set, options, bl)]
struct Settings;

#[group]
#[prefixes("blacklist", "bl")]
#[commands(remove, add, blacklist_show)]
struct Blacklist;

#[help]
// This replaces the information that a user can pass
// a command-name as argument to gain specific information about it.
#[individual_command_tip = "Hello! こんにちは！Hola! Bonjour! 您好! 안녕하세요~\n\n\
I'm a bot that helps protect servers against raids. For now, I am not hosted constantly and only people with admin perms can use my best commands"]
// Some arguments require a `{}` in order to replace it with contextual information.
// In this case our `{}` refers to a command's name.
#[command_not_found_text = "Could not find: `{}`."]
// Define the maximum Levenshtein-distance between a searched command-name
// and commands. If the distance is lower than or equal the set distance,
// it will be displayed as a suggestion.
// Setting the distance to 0 will disable suggestions.
#[max_levenshtein_distance(3)]
// On another note, you can set up the help-menu-filter-behaviour.
// Here are all possible settings shown on all possible options.
// First case is if a user lacks permissions for a command, we can hide the command.
#[lacking_permissions = "Hide"]
// If the user is nothing but lacking a certain role, we just display it hence our variant is `Nothing`.
#[lacking_role = "Nothing"]
// The last `enum`-variant is `Strike`, which ~~strikes~~ a command.
//#[wrong_channel = "Nothing"]
// Serenity will automatically analyse and generate a hint/tip explaining the possible
// cases of ~~strikethrough-commands~~, but only if
// `strikethrough_commands_tip_in_{dm, guild}` aren't specified.
// If you pass in a value, it will be displayed instead.
#[strikethrough_commands_tip_in_guild = ""]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
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
        .help(&MY_HELP)
        .group(&GENERAL_GROUP)
        .group(&BLACKLIST_GROUP)
        .group(&SETTINGS_GROUP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .intents(
            GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::privileged(),
        ) // TODO: more of these!
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
