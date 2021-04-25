use crate::db::{Action, MyDbContext, Settings};
use rand::seq::SliceRandom;
use serenity::framework::standard::CommandResult;
use serenity::prelude::{Context, SerenityError};
use std::collections::HashMap;
use std::convert::TryInto;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use serenity::model::channel::PrivateChannel;
use serenity::model::event::EventType::GuildBanAdd;
use serenity::model::guild::VerificationLevel;
use serenity::model::id::{ChannelId, GuildId};
use serenity::{async_trait, client::bridge::gateway::{ShardId, ShardManager}, framework::standard::{
    buckets::{LimitedFor, RevertBucket},
    help_commands,
    macros::{check, command, group, help, hook},
    Args, CommandGroup, CommandOptions, DispatchError, HelpOptions, Reason, StandardFramework,
}, http::Http, model::{
    channel::{Channel, GuildChannel, Message},
    gateway::Ready,
    guild::Guild,
    id::UserId,
    permissions::Permissions,
    user::User,
}, prelude::Mentionable, utils::{content_safe, ContentSafeOptions}, http};
use std::collections::hash_map::Drain;
use std::borrow::Cow;
/*

all times are stored as ms since unix epoch, as u64

Next steps:
  transient states
   - add event driven checks of status and purging cache
   - better help




some time after raid stops, turn panic mode off


*/

pub async fn check_against_pings(ctx: &Context, mom: &mut YourMama, guild: u64) {
    //println!("{:?} {:?} I am totally checking for people pinging too much here", guild, ctx.author.id);
}

pub async fn check_against_blacklist(
    ctx: &Context,
    mut member: serenity::model::guild::Member,
    guild: u64,
) {
    //println!("I am totally checking against the blacklist here");
    let mut data = ctx.data.write().await;
    let dbcontext = data
        .get::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let settings = dbcontext.get_settings(&guild).await.unwrap();
    let regex_match = settings.blacklist.regex_name_matches(&member.user.name);
    let simple_match = settings.blacklist.simplename.contains(&member.user.name);
    let avatar_match = (member.user.avatar.is_some()
        && settings
            .blacklist
            .avatar
            .contains(&member.user.avatar.as_ref().unwrap()));
    let reason = {
        if avatar_match {
            "avatar"
        } else if regex_match {
            "username by regex"
        } else {
            // simple_match must be true
            "simple username"
        }
    };
    if regex_match || simple_match || avatar_match {
        let outcome = match settings.blacklistaction {
            Action::Ban => {
                let _ = member.user.direct_message(&ctx.http, |f| f.content("Hey bud. You tried to join a server but they don't like your name or pfp so you got banned.")).await;
                let res = GuildId(guild)
                    .ban_with_reason(
                        &ctx.http,
                        &member,
                        0,
                        format!("User matched the {} blacklist", reason),
                    )
                    .await;
                ("Banned", "ban", res)
            }
            Action::Kick => {
                let _ = member.user.direct_message(&ctx.http, |f| f.content("Hey bud. You tried to join a server but they don't like your name or pfp so you got booted.")).await;
                let res = GuildId(guild)
                    .kick_with_reason(
                        &ctx.http,
                        &member,
                        &format!("User matched the {} blacklist", reason),
                    )
                    .await;
                ("Kicked", "kick", res)
            }
            Action::Mute => {
                let _ = member.user.direct_message(&ctx.http, |f| f.content("Hey bud. You tried to join a server but they don't like your name or pfp so you got muted.")).await;
                let res = member.add_role(ctx, settings.muteroll).await;
                ("Muted", "mute", res)
            }
            Action::Nothing => {
                ChannelId(settings.logs).say(&ctx.http, format!("New member {}#{} (id {2} - <@{2}>) matches the {3} blacklist, but settings tell me to do nothing about it.", &member.user.name, &member.user.discriminator, member.user.id, reason)).await;
                return;
            }
        };
        match outcome.2 {
            Ok(_) => {
                if settings.logs > 0 {
                    ChannelId(settings.logs)
                        .say(
                            &ctx.http,
                            format!(
                                "{} {}#{} (id {3} - <@{3}>) because they matches the {4} blacklist",
                                outcome.0,
                                &member.user.name,
                                &member.user.discriminator,
                                member.user.id,
                                reason
                            ),
                        )
                        .await;
                }
            }
            Err(why) => {
                if settings.logs > 0 {
                    ChannelId(settings.logs).say(&ctx.http, format!("Tried and failed to {} {}#{} (id {3} - <@{3}>) because their name or avatar matches the {4} blacklist. Please grant me more permissions so I can do my job better", outcome.1, &member.user.name, &member.user.discriminator, member.user.id, reason)).await;
                }
                println!("{}", why)
            }
        };
    }
}

pub async fn check_against_joins(ctx: &Context, guild: u64) {
    println!("I am totally checking for people joining too much here");
    let mut data = ctx.data.write().await;
    let dbcontext = data
        .get::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let settings = dbcontext.get_settings(&guild).await.unwrap().clone();
    let max_users = settings.users as u64;
    let max_time = settings.time;

    let mut grammy = data
        .get_mut::<Gramma>()
        .expect("Expected your momma in TypeMap.");
    let mut mom = grammy.get(&guild);

    let now = time_now();
    let mut n = 0u64;
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
        start_panicking(&ctx, mom, &settings, guild, false).await;
    }

    if mom.panicking {
        let g = GuildId(guild)
            .to_guild_cached(&ctx.cache)
            .await
            .expect("it should be cached bruh");
        let user = UserId(*mom.recent_users.get(&latest_joiner_ts).expect("broski "));
        mom.yeeted.push(user.0);
        let dms: Option<PrivateChannel> = match user.create_dm_channel(&ctx.http).await {
            Ok(channel) => Some(channel),
            Err(why) => {
                println!(
                    "Couldn't make dm channel to apologize to {} in guild {}",
                    user.0, guild
                );
                None
            }
        };
        match settings.action {
            Action::Ban => {
                if let Some(channel) = dms {
                    channel.say(&ctx.http, "Unfortunately, the server you are trying to join is being raided, and you have been banned").await;
                }
                g.ban_with_reason(&ctx.http, user, 0, "auto ban: joined while raid ongoing")
                    .await;
                // ban them
            }
            Action::Kick => {
                if let Some(channel) = dms {
                    channel.say(&ctx.http, "Unfortunately, the server you are trying to join is being raided, and you have been kicked").await;
                }
                g.kick_with_reason(&ctx.http, user, "auto kick: joined while raid ongoing")
                    .await;
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
            _ => (),
        };
    }
}

/// Check if if we are more than `dt_seconds` past a past timestamp
pub fn time_is_past(start: u64, dt_seconds: u64) -> bool {
    time_now() > start + 1000 * dt_seconds
}

pub(crate) async fn stop_panicking(
    ctx: &Context,
    mom: &mut YourMama,
    settings: &Settings,
    guild_id: u64,
) {
    println!("We stopping the panic broooooo");
    mom.panicking = false;
    if settings.logs > 0 {
        match ChannelId(settings.logs)
            .say(&ctx.http, format!("Panic mode has been deactivated. I brought justice upon {} users.", mom.yeeted.len()))
            .await
        {
            Ok(_) => (),
            Err(why) => println!("error printing stuff: {}", why),
        }

        if !mom.yeeted.is_empty() {
            // Write the ids of the raiders in a file, and attach it in chat
            let string = mom.yeeted.iter().fold(String::new(), |to_return, id| format!("{} {}", to_return, id));
            let attachment = http::AttachmentType::Bytes {
                filename: String::from("Raiders.txt"),
                data: Cow::from(string.into_bytes())
            };

            ChannelId(settings.logs).send_files(&ctx, vec![attachment], |m| {
                m.content("These are the raiders")
            }).await;
        }
    }
    GuildId(guild_id)
        .edit(&ctx.http, |g| {
            g.verification_level(mom.normal_verification_level)
        })
        .await;
    mom.yeeted = vec![];   // clear for next time
}

pub(crate) async fn start_panicking(
    ctx: &Context,
    mom: &mut YourMama,
    settings: &Settings,
    guild_id: u64,
    manually_started: bool,
) {
    println!("We starting to panic broooooo");
    mom.panicking = true;
    match ctx.cache.guild(guild_id).await {
        Some(g) => mom.normal_verification_level = g.verification_level,
        None => println!("bruh problemssaoheunstahoesnuta"),
    }


    let mut g = GuildId(guild_id);
    match g
        .edit(&ctx.http, |g| {
            g.verification_level(VerificationLevel::Higher)
        })
        .await
    {
        Ok(_) => println!("set verification level of {} to High", guild_id),
        Err(why) => println!("Error setting verification level of {}: {}", guild_id, why),
    };


    if settings.logs > 0 {
        let msg = if 0 < settings.notify && !manually_started {
            format!("bruh <@&{}> we are under a tack", settings.notify)
        } else {
            String::from("Bruh we are under a tack")
        };
        match ChannelId(settings.logs)
            .say(&ctx.http, msg)
            .await
        {
            Ok(_) => (),
            Err(why) => println!("error printing stuff in guild {}: {}", guild_id, why),
        }
    }
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
    pub yeeted: Vec<u64>,
    pub normal_verification_level: VerificationLevel,
}

impl YourMama {
    pub fn new() -> Self {
        YourMama {
            recent_users: HashMap::new(),
            userpings: HashMap::new(),
            rollpings: HashMap::new(),
            panicking: false,
            yeeted: vec![],
            normal_verification_level: VerificationLevel::Low,
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
