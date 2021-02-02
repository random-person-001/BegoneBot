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
mod db;
use crate::autopanic::*;
use std::convert::TryInto;
use std::time::{SystemTime, UNIX_EPOCH};
use std::process::exit;

// A container type is created for inserting into the Client's `data`, which
// allows for data to be accessible across all events and framework commands, or
// anywhere else that has a copy of the `data` Arc.
struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
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
        println!("Message! {}", &new_message.content);
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

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(about, ping, upper_command, invite, die)]
struct General;

#[group]
// Sets multiple prefixes for a group.
// This requires us to call commands in this group
// via `~emoji` (or `~em`) instead of just `~`.
#[prefixes("emoji", "em")]
// Set a description to appear if a user wants to display a single group
// e.g. via help using the group-name or one of its prefixes.
#[description = "A group with commands providing an emoji as response."]
// Summary only appears when listing multiple groups.
#[summary = "Do emoji fun!"]
// Sets a command that will be executed if only a group-prefix was passed.
#[default_command(bird)]
#[commands(cat, dog)]
struct Emoji;

#[group]
// Sets a single prefix for this group.
// So one has to call commands in this group
// via `~math` instead of just `~`.
#[prefix = "math"]
#[commands(multiply)]
struct Math;

#[group]
#[prefix = "autopanic"]
#[commands(
    current,
    action,
    reset,
    users,
    time,
    enable,
    disable,
    logs,
    setmuteroll,
    uinfo
)]
struct AutoPanic;

#[help]
// This replaces the information that a user can pass
// a command-name as argument to gain specific information about it.
#[individual_command_tip = "Hello! こんにちは！Hola! Bonjour! 您好! 안녕하세요~\n\n\
If you want more information about a specific command, just pass the command as argument."]
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
#[wrong_channel = "Strike"]
// Serenity will automatically analyse and generate a hint/tip explaining the possible
// cases of ~~strikethrough-commands~~, but only if
// `strikethrough_commands_tip_in_{dm, guild}` aren't specified.
// If you pass in a value, it will be displayed instead.
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
                .prefix("&")
                .case_insensitivity(true)
                .allow_dm(false)
        })
        .unrecognised_command(unknown_command)
        // Set a function that's called whenever a command's execution didn't complete for one
        // reason or another. For example, when a user has exceeded a rate-limit or a command
        // can only be performed by the bot owner.
        .on_dispatch_error(dispatch_error)
        // Can't be used more than once per 5 seconds:
        .bucket("emoji", |b| b.delay(5))
        .await
        // The `#[group]` macro generates `static` instances of the options set for the group.
        // They're made in the pattern: `#name_GROUP` for the group instance and `#name_GROUP_OPTIONS`.
        // #name is turned all uppercase
        .help(&MY_HELP)
        .group(&GENERAL_GROUP)
        .group(&EMOJI_GROUP)
        .group(&MATH_GROUP)
        .group(&AUTOPANIC_GROUP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .intents(GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::privileged()) // TODO: more of these!
        .await
        .expect("Err creating client");

    {
        let conn = sqlx::SqlitePool::connect("db.sqlite").await;
        let mut data = client.data.write().await;
        data.insert::<db::MyDbContext>(MyDbContext::new(conn.unwrap()));
        data.insert::<autopanic::Gramma>(autopanic::Gramma::new());
        data.insert::<CommandCounter>(HashMap::default());
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

#[command]
async fn invite(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    msg.channel_id.say(&ctx.http, "<https://discordapp.com/oauth2/authorize?client_id=802019556801511424&scope=bot&permissions=18502>").await?;
    Ok(())
}

#[command]
async fn die(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if msg.author.id.0 == 275384719024193538 {
        msg.channel_id.say(&ctx.http, "ok bye").await?;
        ctx.shard.shutdown_clean();
        exit(0);
        // todo: make this work more
    } else {
        msg.channel_id.say(&ctx.http, "shut up pleb").await?;
    }
    Ok(())
}
#[command]
// Limits the usage of this command to roles named:
#[allowed_roles("mods", "ultimate neko")]
async fn about_role(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let potential_role_name = args.rest();

    if let Some(guild) = msg.guild(&ctx.cache).await {
        // `role_by_name()` allows us to attempt attaining a reference to a role
        // via its name.
        if let Some(role) = guild.role_by_name(&potential_role_name) {
            if let Err(why) = msg
                .channel_id
                .say(&ctx.http, &format!("Role-ID: {}", role.id))
                .await
            {
                println!("Error sending message: {:?}", why);
            }

            return Ok(());
        }
    }

    msg.channel_id
        .say(
            &ctx.http,
            format!("Could not find role named: {:?}", potential_role_name),
        )
        .await?;

    Ok(())
}

#[command]
// Lets us also call `~math *` instead of just `~math multiply`.
#[aliases("*")]
async fn multiply(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let first = args.single::<f64>()?;
    let second = args.single::<f64>()?;

    let res = first * second;

    msg.channel_id.say(&ctx.http, &res.to_string()).await?;

    Ok(())
}

#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .say(&ctx.http, "This is a small antiraid bot in rust")
        .await?;

    Ok(())
}

#[command]
// Limit command usage to guilds.
#[only_in(guilds)]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong! : )").await?;

    Ok(())
}

#[command]
// Make this command use the "emoji" bucket.
#[bucket = "emoji"]
// Allow only administrators to call this:
#[required_permissions("ADMINISTRATOR")]
async fn cat(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, ":cat:").await?;

    // We can return one ticket to the bucket undoing the ratelimit.
    Err(RevertBucket.into())
}

#[command]
#[description = "Sends an emoji with a dog."]
#[bucket = "emoji"]
async fn dog(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, ":dog:").await?;

    Ok(())
}

#[command]
async fn bird(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let say_content = if args.is_empty() {
        ":bird: can find animals for you.".to_string()
    } else {
        format!(":bird: could not find animal named: `{}`.", args.rest())
    };

    msg.channel_id.say(&ctx.http, say_content).await?;

    Ok(())
}

// We could also use
// #[required_permissions(ADMINISTRATOR)]
// but that would not let us reply when it fails.
#[command]
async fn am_i_admin(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    if let Some(member) = &msg.member {
        for role in &member.roles {
            if role
                .to_role_cached(&ctx.cache)
                .await
                .map_or(false, |r| r.has_permission(Permissions::ADMINISTRATOR))
            {
                msg.channel_id.say(&ctx.http, "Yes, you are.").await?;

                return Ok(());
            }
        }
    }

    msg.channel_id.say(&ctx.http, "No, you are not.").await?;

    Ok(())
}

#[command]
async fn slow_mode(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let say_content = if let Ok(slow_mode_rate_seconds) = args.single::<u64>() {
        if let Err(why) = msg
            .channel_id
            .edit(&ctx.http, |c| c.slow_mode_rate(slow_mode_rate_seconds))
            .await
        {
            println!("Error setting channel's slow mode rate: {:?}", why);

            format!(
                "Failed to set slow mode to `{}` seconds.",
                slow_mode_rate_seconds
            )
        } else {
            format!(
                "Successfully set slow mode rate to `{}` seconds.",
                slow_mode_rate_seconds
            )
        }
    } else if let Some(Channel::Guild(channel)) = msg.channel_id.to_channel_cached(&ctx.cache).await
    {
        format!(
            "Current slow mode rate is `{}` seconds.",
            channel.slow_mode_rate.unwrap_or(0)
        )
    } else {
        "Failed to find channel in cache.".to_string()
    };

    msg.channel_id.say(&ctx.http, say_content).await?;

    Ok(())
}

// A command can have sub-commands, just like in command lines tools.
// Imagine `cargo help` and `cargo help run`.
#[command("upper")]
#[sub_commands(sub)]
async fn upper_command(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "This is the main function!").await?;

    Ok(())
}

// This will only be called if preceded by the `upper`-command.
#[command]
#[aliases("sub-command", "secret")]
#[description("This is `upper`'s sub-command.")]
async fn sub(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    msg.reply(&ctx.http, "This is a sub function!").await?;

    Ok(())
}
