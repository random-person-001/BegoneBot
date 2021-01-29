use crate::db::{Action, MyDbContext, Settings};
use rand::seq::SliceRandom;
use serenity::framework::standard::CommandResult;
use serenity::prelude::Context;
use std::collections::HashMap;
use std::convert::TryInto;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use serenity::{
    prelude::Mentionable,
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
        channel::{Channel, Message, GuildChannel},
        gateway::Ready,
        guild::Guild,
        id::UserId,
        permissions::Permissions,
    },
    utils::{content_safe, ContentSafeOptions},
};
/*

all times are stored as ms since unix epoch, as u64

Next steps:
  transient states
   - add event driven checks of status and purging cache
   - actual punishment and changy stuff
   - better help
   - better settings changing


  - set status to D&D

? / help      display help
.on/off        enable/disable auto panic
.current       display current settings
.users <#>     User joins needed to trigger AP
.time <#>      time window to trigger AP
.action        choose whether to ban, kick, or mute upon AP
now           turns on panic mode immediately
stop          turns off panic mode immediately
.muteroll @r   sets @r to be the roll applied automatically to noobs during panic when action=mute
.logs #chan    sets #chan to be where logs occur

- Dm autokicked/banned people with message like "we currently do not allow new people in the server because we are being raided, try again later", or something like that.

. - ability to get detailed and specific userinfo even if person isn't in the server (like avatar, username)


.roll/user/any mention spam limit

blacklist     automatically apply `action` to any new join at any time if they have a bad -
 name simple  username
 name regex   username
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

pub fn check_against_pings(ctx: &Context, mom: &mut YourMama, guild: u64) {
    println!("I am totally checking for people pinging too much here");
}

pub fn check_against_joins(ctx: &Context, mom: &mut YourMama, guild: u64) {
    println!("I am totally checking for people joining too much here");
}

/// Check if if we are more than `dt_seconds` past a past timestamp
pub fn time_is_past(start: u64, dt_seconds: u64) -> bool {
    return time_now() > start + 1000 * dt_seconds;
}

/// Return ms since epoch
pub fn time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
        .try_into()
        .unwrap()
}

#[derive(Debug)]
pub struct Gramma {
    pub guild_mamas: HashMap<u64, YourMama>,
}

impl Gramma {
    pub fn new() -> Self {
        Gramma {
            guild_mamas: HashMap::new(),
        }
    }

    pub fn get(&mut self, guild: &u64) -> &mut YourMama {
        return self.guild_mamas.entry(*guild).or_insert(YourMama::new());
    }
}

/// Store of transient info for the bot
#[derive(Debug)]
pub struct YourMama {
    pub recent_users: HashMap<u64, u64>,       // time joined / user
    pub userpings: HashMap<u64, (usize, u64)>, // timestamp, number of pings, user
    pub rollpings: HashMap<u64, (usize, u64)>, // timestamp, number of pings, user
    pub panicking: bool,
    pub panick_end: i64,
}

impl YourMama {
    pub fn new() -> Self {
        YourMama {
            recent_users: HashMap::new(),
            userpings: HashMap::new(),
            rollpings: HashMap::new(),
            panicking: false,
            panick_end: -1,
        }
    }
}

/// for getting a random sadface
struct Sads {
    pub maxpogs: Vec<String>,
}

impl Sads {
    pub fn new() -> Self {
        Sads {
            maxpogs: vec![
                String::from("<:maxpog1:804177392775331863>"),
                String::from("<:maxpog2:804178291866861578>"),
                String::from("<:maxpog3:804179033351782441>"),
            ],
        }
    }

    pub fn get_one(&self) -> &str {
        &self.maxpogs.choose(&mut rand::thread_rng()).unwrap()[..]
    }
}

fn humanize_duration(mut secs: i64) -> String {
    println!("{}", secs);
    let s = secs % 60;
    secs -= s;
    let m = (secs % 3600) / 60;
    secs -= m * 60;
    let h = (secs % (3600*24)) / 3600;
    secs -= h * (3600);
    let d = (secs % (3600*24*365)) / (3600*24);
    secs -= d * (3600 * 24);
    let y = secs/(3600 * 24 * 365);
    format!("{}y {}d {}h {}m {}s", y, d, h, m, s)
}


#[command]
/// Query discord for all the info we can find about a given user, specified by id
async fn uinfo(ctx: &Context, msg: &Message, args:Args) -> CommandResult {
    let user_id = UserId(match args.clone().single::<u64>() {
        Ok(n) => n,
        Err(why) => {
            msg.channel_id.say(&ctx.http, "Sorry, that wasn't recognized as a reasonable discord user id.").await?;
            return Ok(());
        }
    });
    match user_id.to_user(&ctx.http).await {
        Ok(u) => {
            let account_creation = u.created_at().naive_utc().format("%Y-%m-%d %H:%M:%S").to_string();
            let now:i64 = time_now().try_into().unwrap();
            let age = humanize_duration(now/1000 - u.created_at().naive_utc().timestamp());
            let msg = msg.channel_id.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title(u.tag());
                    e.description(format!("{}\nAccount created on {}\nwhich is {} ago", u.mention(), account_creation, age));
                    e.thumbnail(u.face());
                    e.footer(|f| {
                        f.text(format!("User id = {}", user_id.0));
                        f
                    });
                    e
                });
                m
            }).await;
            ()
        }
        Err(why) => {
            msg.channel_id.say(&ctx.http, "No user with that id could be found").await;
            ()
        },
    };

    Ok(())
}

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
    let choice = match &choice[..] {
        "ban" => Action::Ban,
        "kick" => Action::Kick,
        "mute" => Action::Mute,
        "nothing" => Action::Nothing,
        default => {
            let s = "Sorry that wasn't recognized. Recognized options are ban, kick, or mute";
            msg.channel_id.say(&ctx.http, s).await?;
            return Ok(());
        }
    };
    dbcontext
        .set_attr(&guild, "action", choice.try_into().unwrap())
        .await;

    msg.channel_id.say(&ctx.http, "Updated.").await?;
    if let Some(settings) = dbcontext.fetch_settings(&guild).await {
        if &choice == &Action::Mute && settings.muteroll == 0 {
            let mut roll: u64 = 0;
            if let Some(rol) = get_roll_id_by_name(&ctx, &msg, "muted").await {
                roll = rol;
            } else if let Some(rol) = get_roll_id_by_name(&ctx, &msg, "Muted").await {
                roll = rol;
            }

            if roll > 0 {
                let s = "Automatically detected the Muted roll here";
                msg.channel_id.say(&ctx.http, s).await?;
                dbcontext
                    .set_attr(&guild, "muteroll", roll.try_into().unwrap())
                    .await;
            } else {
                let s = "Please specify which roll to give members to mute them by running antiraid setmuteroll @thatroll. Until then I cannot help you in a raid.";
                msg.channel_id.say(&ctx.http, s).await?;
            }
        }
    } else {
        println!("uh no settings found");
    }
    Ok(())
}

#[command]
async fn reset(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };

    if dbcontext.add_guild(&guild).await {
        msg.channel_id
            .say(&ctx.http, "Successfully reset settings")
            .await?;
    } else {
        let s = "Problems were encountered while attempting to reset settings";
        msg.channel_id.say(&ctx.http, s).await?;
    }
    Ok(())
}

#[command]
async fn current(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };

    let settings = dbcontext.fetch_settings(&guild).await;
    let settings = settings.unwrap();

    // embeds -
    // https://github.com/serenity-rs/serenity/blob/current/examples/e09_create_message_builder/src/main.rs
    let s = format!(
        r#"**Automatic antiraid settings**
    Automatic raid detection is currently __{}__
    Panic mode is automatically triggered when __{}__ users join in __{}__ seconds.
    During panic mode:
      - server verification level will be turned to Highest (verified phone required to join)
      - any member joining will be {}

    **General settings**
    Ping spam limits are {}.
    {}.
    "#,
        if settings.enabled {
            "**DISABLED!**"
        } else {
            "ENABLED."
        },
        settings.users,
        settings.time,
        match settings.action {
            Action::Ban => String::from("dmed an explanation and __banned__."),
            Action::Kick => String::from("dmed an explanation and __kicked__."),
            Action::Mute => {
                if settings.muteroll == 0 {
                    String::from("dmed an explanation and __muted.__\n    **PLEASE TELL ME WHAT ROLL TO GIVE PEOPLE TO MUTE THEM** - run `autopanic setmuteroll @theroll`")
                } else {
                    format!("__muted__ (given __<@&{}>__)", settings.muteroll)
                }
            }
            Action::Nothing => String::from("__left alone__ by me"),
        },
        if Action::Nothing == settings.mentionaction {
            String::from("__disabled__")
        } else {
            format!("__enabled__:\n      - Members will be __{}__ if they ping __{}__ users, __{}__ mentions, or __{}__ of either within __{}__ seconds", match settings.mentionaction {
                Action::Ban => "banned",
                Action::Kick => "kicked",
                Action::Mute => "muted",
                Action::Nothing => "this state is not reachable in code",
            }, settings.usermentions, settings.rollmentions, settings.anymentions, settings.mentiontime)
        },
        if let 0 = settings.logs {
            String::from("__No__ logging channel is configured")
        } else {
            format!(
                "Logs are posted in __<#{}>__ \n    __{}__ is pinged when a raid is detected",
                settings.logs,
                match settings.notify {
                    0 => String::from("no roll"),
                    n => format!("<@&{}>", n),
                }
            )
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
    if dbcontext
        .set_attr(&guild, "users", choice.try_into().unwrap())
        .await
    {
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
    if dbcontext
        .set_attr(&guild, "time", choice.try_into().unwrap())
        .await
    {
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
    let choice = match id_from_mention(&choice[..]) {
        Some(id) => id,
        None => {
            let s = "Bro that didn't look like a normal channel message :(";
            msg.channel_id.say(&ctx.http, s).await?;
            return Ok(());
        }
    };

    if dbcontext
        .set_attr(&guild, "logs", choice.try_into().unwrap())
        .await
    {
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
async fn setmuteroll(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };
    let choice = args.clone().single::<String>().unwrap();

    let roll: u64;
    if let Some(rol) = get_roll_id_by_name(&ctx, &msg, &choice[..]).await {
        roll = rol;
    } else if let Some(rol) = get_roll_id_by_name(&ctx, &msg, &choice.to_lowercase()[..]).await {
        roll = rol;
    } else if let Some(rol) = id_from_mention(&choice[..]) {
        roll = rol;
    } else {
        let s = "Broski that didn't look like a roll that you have here :(";
        msg.channel_id.say(&ctx.http, s).await?;
        return Ok(());
    };

    if dbcontext
        .set_attr(&guild, "muteroll", roll.try_into().unwrap())
        .await
    {
        msg.channel_id.say(&ctx.http, "Updated.").await?;
        Ok(())
    } else {
        let s = "Problems arose when trying to update settings.";
        msg.channel_id.say(&ctx.http, s).await?;
        Ok(())
    }
}

#[command]
async fn enable(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };
    if dbcontext.set_enabled(&guild, true).await {
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
async fn disable(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0.try_into().unwrap(),
        None => 0,
    };
    if dbcontext.set_enabled(&guild, false).await {
        msg.channel_id.say(&ctx.http, "Updated.").await?;
        Ok(())
    } else {
        msg.channel_id
            .say(&ctx.http, "Problems arose when trying to update settings.")
            .await?;
        Ok(())
    }
}

/// Retrieves the nth `char` of a `&str`
/// If `n < 0` then it will always retrieve the final `char`
/// If you attempt to index out of bounds, it will return a ` `
fn nth_char(s: &str, n: isize) -> char {
    if n < 0 {
        return match s.chars().last() {
            Some(c) => c,
            None => ' ',
        };
    };
    match s.chars().nth(n.try_into().unwrap()) {
        Some(c) => c,
        None => ' ',
    }
}

/// Extract a discord ID (`u64`) from a word that is a discord mention
/// This could be like a channel <#numbers>, a user <@numbers>, or a roll <@&numbers>
fn id_from_mention(s: &str) -> Option<u64> {
    println!("{}", s);
    if nth_char(&s, 0) != '<' && nth_char(&s, -1) != '>' {
        // todo: work if just a str id is inputted as well
        return None;
    }
    let num: &str;
    if nth_char(&s, 2) == '!' || nth_char(&s, 2) == '&' {
        // latter case is for roll mentions
        num = &s[3..s.len() - 1];
    } else {
        num = &s[2..s.len() - 1];
    }

    match num.parse::<u64>() {
        Ok(num) => Some(num),
        Err(_) => None,
    }
}

async fn get_roll_id_by_name(ctx: &Context, msg: &Message, name: &str) -> Option<u64> {
    if let Some(guild_id) = msg.guild_id {
        if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
            if let Some(role) = guild.role_by_name(name) {
                return Some(role.id.0);
            }
        }
    }
    None
}
