use crate::db::{Action, MyDbContext, Settings};
use rand::seq::SliceRandom;
use serenity::framework::standard::CommandResult;
use serenity::prelude::{Context, SerenityError};
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
        user::User,
    },
    utils::{content_safe, ContentSafeOptions},
};
use serenity::model::event::EventType::GuildBanAdd;
use serenity::model::id::GuildId;
use serenity::model::guild::VerificationLevel;
use serenity::model::channel::PrivateChannel;
/*

all times are stored as ms since unix epoch, as u64

Next steps:
  transient states
   - add event driven checks of status and purging cache
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

.- Dm auto kicked/banned people with message like "we currently do not allow new people in the server because we are being raided, try again later", or something like that.

.ability to get detailed and specific userinfo even if person isn't in the server (like avatar, username)


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

pub async fn check_against_pings(ctx: &Context, mom: &mut YourMama, guild: u64) {
    println!("I am totally checking for people pinging too much here");
}

pub async fn check_against_joins(ctx: &Context, guild: u64) {
    println!("I am totally checking for people joining too much here");
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let settings = dbcontext.fetch_settings(&guild).await.unwrap();

    let mut grammy = data
        .get_mut::<Gramma>()
        .expect("Expected your momma in TypeMap.");
    let mut mom = grammy.get(&guild);

    let max_users = settings.users as u64;
    let max_time = settings.time;
    let now = time_now();
    let mut n= 0u64;
    let mut latest_joiner_ts = 0u64;
    for (&timestamp, &user) in &mom.recent_users {
        if now - timestamp < (max_time * 1000) as u64 {
            n += 1;
            if timestamp > latest_joiner_ts {
                latest_joiner_ts = timestamp;
            }
        }
    }
    println!("{:?}", mom);
    if n < max_users {
        return;
    }
    println!("Yeetus we panic");
    // PANIC!  WE'RE BEING RAIDED.  this is the turn-panic-on clause
    if !mom.panicking {
        start_panicking(&ctx, mom, &settings, guild).await;
    }
    mom.panic_end = now as i64 + (max_time * 1000 * 4) as i64;  // todo: note implementation here

    if mom.panicking {
        let g = GuildId(guild).to_guild_cached(&ctx.cache).await.expect("it should be cached bruh");
        let user = UserId(*mom.recent_users.get(&latest_joiner_ts).expect("broski "));
        let dms:Option<PrivateChannel> = match user.create_dm_channel(&ctx.http).await {
            Ok(channel) => Some(channel),
            Err(why) => {
                println!("Couldn't make dm channel to apologize to {} in guild {}", user.0, guild);
                None
            }
        };
        match settings.action {
            Action::Ban => {
                if let Some(channel) = dms {
                    channel.say(&ctx.http, "Unfortunately, the server you are trying to join is being raided, and you have been banned").await;
                }
                g.ban_with_reason(&ctx.http, user, 0, "auto ban: joined while raid ongoing").await;
                // ban them
            }
            Action::Kick => {
                if let Some(channel) = dms {
                    channel.say(&ctx.http, "Unfortunately, the server you are trying to join is being raided, and you have been kicked").await;
                }
                g.kick_with_reason(&ctx.http, user, "auto kick: joined while raid ongoing").await;
                // kick them
            }
            Action::Mute => {
                if settings.muteroll > 0 {
                    if let Some(channel) = dms {
                        channel.say(&ctx.http, "Unfortunately, the server you are trying to join is being raided, and you have been muted").await;
                    }
                    let mut member = match g.member(ctx, user).await {
                        Ok(m) => m,
                        Err(why) => {
                            println!("member may have already yeeted {}", why);
                            return;
                        }
                    };
                    // Assign role
                    match member.add_role(ctx, settings.muteroll).await {
                        Ok(_) => (),
                        Err(why) => {
                            println!("big h moment: {}", why);
                            return;
                        }
                    }
                }
            }
            _ => ()
        };
    } else {}
}

/// Check if if we are more than `dt_seconds` past a past timestamp
pub fn time_is_past(start: u64, dt_seconds: u64) -> bool {
    time_now() > start + 1000 * dt_seconds
}

async fn start_panicking(ctx: &Context, mom: &mut YourMama, settings: &Settings, guild_id: u64) {
    println!("We starting to panic broooooo");
    mom.panicking = true;
    if let Some(channel) = ctx.cache.guild_channel(guild_id).await {
        if 0 < settings.notify {
            let r = channel.say(&ctx.http, format!("bruh <@&{}> we are under a tack", settings.notify)).await;
        } else {
            channel.say(&ctx.http, "Bruh we are under a tack").await;
        }
    }

    let mut g = GuildId(guild_id);
    match g.edit(&ctx.http, |g| {
        g.verification_level(VerificationLevel::Higher)
    }).await {
        Ok(_) => println!("set verification level of {} to High", g.0),
        Err(why) => println!("Error setting verification level of {}: {}", g.0, why),
    };
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
        return self.guild_mamas.entry(*guild).or_insert_with(YourMama::new);
    }
}

/// Store of transient info for the bot
#[derive(Debug)]
pub struct YourMama {
    pub recent_users: HashMap<u64, u64>,       // time joined / user
    pub userpings: HashMap<u64, (usize, u64)>, // timestamp, number of pings, user
    pub rollpings: HashMap<u64, (usize, u64)>, // timestamp, number of pings, user
    pub panicking: bool,
    pub panic_end: i64,
}

impl YourMama {
    pub fn new() -> Self {
        YourMama {
            recent_users: HashMap::new(),
            userpings: HashMap::new(),
            rollpings: HashMap::new(),
            panicking: false,
            panic_end: -1,
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
