use crate::autopanic;
use crate::autopanic::Gramma;
use crate::db::{Action, MyDbContext};
use serenity::{
    framework::standard::{
        macros::{check, command},
        Args, CommandResult,
    },
    model::prelude::{Mentionable, Message, Permissions, UserId},
    prelude::{Context, SerenityError},
};
use std::convert::TryInto;
use std::process::exit;

#[command]
async fn die(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if msg.author.id.0 == 275384719024193538 {
        msg.channel_id.say(&ctx.http, "ok bye").await?;
        ctx.shard.shutdown_clean();
        exit(0);
    } else {
        msg.channel_id
            .say(&ctx.http, "shut up pleb you're dirt to me")
            .await?;
    }
    Ok(())
}

#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let c = format!(
        "I'm a small antiraid bot in rust written by John Locke#2742 serving {} guilds\n\
        You can invite me with <https://discordapp.com/oauth2/authorize?client_id=802019556801511424&scope=bot&permissions=18503>",
        ctx.cache.guild_count().await
    );
    msg.channel_id.say(&ctx.http, c).await?;
    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[command]
#[required_permissions("ADMINISTRATOR")]
async fn blacklist_show(ctx: &Context, msg: &Message, args:Args) -> CommandResult{
    let data = ctx.data.write().await;
    let dbcontext = data
        .get::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0,
        None => 0,
    };
    let s = &dbcontext.get_settings(&guild).await.expect("hmm").blacklist;

    let msg = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Current Blacklist" );
                e.color(0x070707);
                e.description(" \
                 • To adjust what happens when a member joins who matches a blacklist item, use `bb-settings set blacklistaction` with `ban`, `kick`, `mute`, or `nothing` at the end (default is kick). \n \
                 • To blacklist any users who are named EvilBotnet, run `bb-blacklist add name EvilBotnet` \n \
                 • To blacklist anyone named like DMSpammer4 or DMSpammer87, run `bb-blacklist add regexname DMSpammer\\d+` \
                Regex is extremely powerful, and there are many references on the web to help you. \
                \n
                 • If you'd like to remove an item, `bb-blacklist remove` will be your friend. \n\
                 • If you'd like to remove the first entry of the regex name blacklist, run `bb-blacklist remove regexname 1` \
                \n
                When talking about these lists in commands (like those examples above), call them `name` `regexname` and `avatar` respectively. \
                ");
                e.field("Simple name blacklist",
                format!("(will match username exactly)\n`{:?}`", s.simplename),
                false);
                e.field("Regex name blacklist",
                format!("(will match username by regex)\n`{:?}`", s.regexname),
                false);
                e.field("Avatar blacklist",
                format!("(will match avatar)\n`{:?}`", s.avatar),
                false);
                e
            });
            m
        })
        .await;
    Ok(())
}

#[command]
#[required_permissions("ADMINISTRATOR")]
async fn add(ctx: &Context, msg: &Message, mut args:Args) -> CommandResult{
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0,
        None => 0,
    };
    let mut s = dbcontext.get_settings(&guild).await.expect("hmm").blacklist.clone();


    if args.len() != 2 {
        println!("{:?}", args);
        msg.channel_id.say(&ctx.http, "I need exactly two things specified after this: which list you're adding to, and the item to add.").await;
        return Ok(());
    }

    let which_list  = args.single::<String>().unwrap();
    let new_item  = args.single::<String>().unwrap();

    match &*which_list {
        "name" => s.simplename.push(new_item),
        "regexname" => s.regexname.push(new_item),
        "avatar" => s.avatar.push(new_item),
        _ => {
            msg.channel_id.say(&ctx.http, "Broski....... please specify the blacklist you're trying to add to here. It can be one of `name` `regexname` `avatar`").await;
            return Ok(())
        }
    };
    if dbcontext.save_blacklist(&guild, s).await {
        msg.channel_id.say(&ctx.http, ":+1: Added").await;
    } else {
        msg.channel_id.say(&ctx.http, "There was a problem saving that setting").await;
    }
    Ok(())
}

#[command]
#[required_permissions("ADMINISTRATOR")]
/// usage: bb-blacklist remove nameregex 4 to run settings.blacklist[3].pop()
async fn remove(ctx: &Context, msg: &Message, mut args:Args) -> CommandResult{
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0,
        None => 0,
    };

    if args.len() != 2 {
        println!("{:?}", args);
        msg.channel_id.say(&ctx.http, "I need exactly two things specified after this: which list you're removing from, and the index of the item you want to remove.").await;
        return Ok(());
    }


    let which_list:String = args.single::<String>().unwrap();


    let index = match args.single::<usize>() {
        Ok(i) => i,
        Err(_) => {
            msg.channel_id.say(&ctx.http, "Oops I need the index for ");
            return Ok(());
        }
    };
    let mut s = dbcontext.get_settings(&guild).await.expect("hmm").blacklist.clone();

    let mut removed = String::new();
    match &*which_list {
        "name" => {
            if s.simplename.is_empty() {
                msg.channel_id.say(&ctx.http, "There aren't any entries in the simple name list, so removing one doesn't make sense").await;
                return Ok(());
            } else if index > s.simplename.len() {
                msg.channel_id.say(&ctx.http, format!("There are only {} entries in the simple name list, so you must specify a number between 1 and that.", s.simplename.len())).await;
                return Ok(());
            } else {
                removed = s.simplename.remove(index-1);
            }
        }
        "regexname" => {
            if s.regexname.is_empty() {
                msg.channel_id.say(&ctx.http, "There aren't any entries in the regex name list, so removing one doesn't make sense").await;
                return Ok(());
            } else if index > s.regexname.len() {
                msg.channel_id.say(&ctx.http, format!("There are only {} entries in the regex name list, so you must specify a number between 1 and that.", s.regexname.len())).await;
                return Ok(());
            } else {
                removed = s.regexname.remove(index-1);
            }
        }
        "avatar" => {
            if s.avatar.is_empty() {
                msg.channel_id.say(&ctx.http, "There aren't any entries in the avatar list, so removing one doesn't make sense").await;
                return Ok(());
            } else if index > s.avatar.len() {
                msg.channel_id.say(&ctx.http, format!("There are only {} entries in the avatar list, so you must specify a number between 1 and that.", s.avatar.len())).await;
                return Ok(());
            } else {
                removed = s.avatar.remove(index-1);
            }
        }
        _ => {
            msg.channel_id.say(&ctx.http, "That option was not recognized. Acceptable inputs are `name` `regexname` `avatar`").await;
            return Ok(());
        }
    };
    if dbcontext.save_blacklist(&guild, s).await {
        msg.channel_id.say(&ctx.http, format!("Successfully removed `{}`", removed)).await;
    } else {
        msg.channel_id.say(&ctx.http, "Problems occurred while updating db :_(\n if they persist, tell the dev or try `db-settings reset`").await;
    };
    Ok(())
}

#[command]
#[required_permissions("ADMINISTRATOR")]
async fn reset(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0,
        None => 0,
    };

    if dbcontext.add_guild(&guild).await {
        msg.channel_id
            .say(&ctx.http, "Successfully reset all settings")
            .await?;
    } else {
        let s = "Problems were encountered while attempting to reset all settings";
        msg.channel_id.say(&ctx.http, s).await?;
    }
    Ok(())
}

#[command]
#[required_permissions("BAN_MEMBERS")]
async fn forceban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut successes: u8 = 0;
    let mut fails: u8 = 0;
    while !args.is_empty() {
        match args.single::<u64>() {
            Ok(uid) => {
                msg.guild_id
                    .expect("infallable")
                    .ban_with_reason(
                        &ctx.http,
                        uid,
                        0,
                        format!("Forceban ran by {}", msg.author.name),
                    )
                    .await;
                successes += 1;
            }
            Err(why) => {
                fails += 1;
                args.advance();
            }
        }
    }
    if fails == 0 {
        if successes == 0 {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Please specify some user IDs after the command, separated by spaces",
                )
                .await;
        } else {
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Banned {} users successfully", successes),
                )
                .await;
        }
    } else {
        msg.channel_id
            .say(
                &ctx.http,
                format!(
                    "Banned {} users successfully and {} unsuccessfully (please only specify user ids)",
                    successes, fails
                ),
            )
            .await;
    }
    Ok(())
}

#[command]
/// Query discord for all the info we can find about a given user, specified by id
async fn uinfo(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let user_id = UserId(match args.clone().single::<u64>() {
        Ok(n) => n,
        Err(why) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Sorry, that wasn't recognized as a reasonable discord user id.",
                )
                .await?;
            return Ok(());
        }
    });
    match user_id.to_user(&ctx.http).await {
        Ok(u) => {
            let account_creation = u
                .created_at()
                .naive_utc()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string();
            let now: i64 = autopanic::time_now().try_into().unwrap();
            let age = humanize_duration(now / 1000 - u.created_at().naive_utc().timestamp());
            let msg = msg
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title(u.tag());
                        e.description(format!(
                            "{}\nAccount created on {}\nwhich is {} ago",
                            u.mention(),
                            account_creation,
                            age
                        ));
                        e.thumbnail(u.face());
                        e.footer(|f| {
                            f.text(format!("User id = {}", user_id.0));
                            f
                        });
                        e
                    });
                    m
                })
                .await;
        }
        Err(why) => {
            msg.channel_id
                .say(&ctx.http, "No user with that id could be found")
                .await;
        }
    };

    Ok(())
}

#[command]
#[required_permissions("ADMINISTRATOR")]
async fn panic(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let guild_id = msg.guild_id.expect("infallible").0;

    let dbcontext = data
        .get::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let settings = dbcontext.get_settings(&guild_id).await.unwrap().clone();
    let mut mom = data
        .get_mut::<Gramma>()
        .expect("Expected your momma in TypeMap.")
        .get(&guild_id);

    if args.is_empty() {
        println!("Panic mode manually activated for server {}", guild_id);
        mom.panicking = true;
        msg.channel_id
            .say(
                &ctx.http,
                "Panic mode has been activated. Turn it off with `bb-panic off`",
            )
            .await;
        autopanic::start_panicking(&ctx, mom, &settings, guild_id).await;
        return Ok(());
    }

    let choice = args.clone().single::<String>().unwrap().to_lowercase();
    println!("'{}'", choice);
    match &choice[..] {
        "on" => {
            println!("Panic mode manually activated for server {}", guild_id);
            mom.panicking = true;
            autopanic::start_panicking(&ctx, mom, &settings, guild_id).await;
            msg.channel_id
                .say(
                    &ctx.http,
                    "Panic mode has been activated. Turn it off with `bb-panic off`",
                )
                .await;
        }
        "off" => {
            println!("Panic mode manually deactivated for server {}", guild_id);
            mom.panicking = false;
            autopanic::stop_panicking(&ctx, mom, &settings, guild_id).await;
            msg.channel_id
                .say(
                    &ctx.http,
                    "Panic mode has been deactivated. Turn it on with `bb-panic`",
                )
                .await;
        }
        _ => {
            let m = "Broski... I need you to say `on` or `off` after that (if you don't put anything, I'll assume on)";
            msg.channel_id.say(&ctx.http, m).await;
            ()
        }
    }

    Ok(())
}

#[command]
#[required_permissions("ADMINISTRATOR")]
async fn options(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let s = "    _Example:_ To set the raid panic to trigger when 3 people join in 4 seconds, send these two messages:
`bb-settings set users 3`
`bb-settings set time 4`

_Example:_ To ping a roll named @Staff when a raid starts (assumes log channel has been set):
`bb-settings set notify Staff`

The following are the settings you can adjust:";
    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title("Configuring Settings");
            e.description(s);
            e.field("enabled", "whether automatic raid detection and prevention should occur. Input is either `true` or `false`", false);
            e.field("action", "What to do to to new joining members during a raid. One of `ban` `kick` `mute` `nothing`", false);
            e.field("users", "How many users it takes to trigger a raid panic. A number", false);
            e.field("time", "When `users` join in this amount of seconds, a raid panic is triggered. A number", false);
            e.field("logs", "What channel to post raid notifications. Is none by default. A blue channel name", false);
            e.field("notify", "When a raid panic starts, this roll is pinged in the `logs` channel.  A roll id or mention or written name", false);
            e.field("blacklistaction", "When a new member joins who matches the blacklist, this will be done to them. One of `ban` `kick` `mute` `nothing`", false);
            e.footer(|f| {
                f.text("To check current settings, run just bb-settings")
            });
            e
        })
    }).await;
    Ok(())
}

#[command]
#[required_permissions("ADMINISTRATOR")]
async fn set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.is_empty() {
        msg.channel_id
            .say(
                &ctx.http,
                "I need some arguments. Run `bb-settings options` to see usage",
            )
            .await;
    }
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild_id = msg.guild_id.expect("").0;
    let setting_name = args.single::<String>().unwrap().to_lowercase();

    if args.is_empty() {
        msg.channel_id.say(
            &ctx.http,
            "I need a value to set that to. Run `bb-settings options` for more",
        );
        return Ok(());
    }
    let choice = args.single::<String>().unwrap().to_lowercase();

    let dbval: i64 = match &setting_name[..] {
        "enabled" => match &choice[..] {
            "true" => true as i64,
            "false" => false as i64,
            _ => {
                msg.channel_id
                    .say(
                        &ctx.http,
                        "That value didn't look right. It should be either `true` or `false`",
                    )
                    .await?;
                return Ok(());
            }
        },
        "action" => {
            let choice = match &choice[..] {
                "ban" => Action::Ban,
                "kick" => Action::Kick,
                "mute" => Action::Mute,
                "nothing" => Action::Nothing,
                default => {
                    let s = "Sorry that option wasn't recognized. Recognized options are `ban`, `kick`, `mute`, or `nothing`.";
                    msg.channel_id.say(&ctx.http, s).await?;
                    return Ok(());
                }
            };

            // Check the mute roll and stuff
            if let Some(settings) = dbcontext.get_settings(&guild_id).await {
                if choice == Action::Mute && settings.muteroll == 0 {
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
                            .set_attr(&guild_id, "muteroll", roll.try_into().unwrap())
                            .await;
                    } else {
                        let s = "Please specify which roll to give members to mute them by running `bb-settings set muteroll @theroll`. Until then I cannot help you in a raid.";
                        msg.channel_id.say(&ctx.http, s).await?;
                    }
                }
            } else {
                println!("uh no settings found");
            }
            choice as i64
        }
        "users" => {
            let choice: u8 = match choice.parse() {
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
            choice as i64
        }
        "time" => {
            let choice: u32 = match choice.parse() {
                Ok(n) => n,
                Err(why) => {
                    msg.channel_id.say(&ctx.http, "Sorry, that wasn't recognized as a reasonable amount of time for users to join in.").await?;
                    return Ok(());
                }
            };
            if choice <= 1 {
                msg.channel_id
                    .say(
                        &ctx.http,
                        "Gotta be bigger, so imma say nope to that chief.",
                    )
                    .await?;
                return Ok(());
            }
            choice as i64
        }
        "logs" => {
            let choice = match id_from_mention(&choice[..]) {
                Some(id) => id,
                None => {
                    let s = "Bro that didn't look like a normal channel message :(";
                    msg.channel_id.say(&ctx.http, s).await?;
                    return Ok(());
                }
            };
            choice as i64
        }
        "muteroll" => {
            if let Some(h) = get_roll_from_set_command(&ctx, &msg, &choice).await {
                h
            } else {
                return Ok(());
            }
        }
        "rollmentions" => 0,
        "usermentions" => 0,
        "anymentions" => 0,
        "mentionaction" => 0,
        "mentiontime" => 0,
        "notify" => {
            if let Some(h) = get_roll_from_set_command(&ctx, &msg, &choice).await {
                h
            } else {
                return Ok(());
            }
        }
        "blacklistaction" => {
            let choice = match &choice[..] {
                "ban" => Action::Ban,
                "kick" => Action::Kick,
                "mute" => Action::Mute,
                "nothing" => Action::Nothing,
                default => {
                    let s = "Sorry that option wasn't recognized. Recognized options are `ban`, `kick`, `mute`, or `nothing`.";
                    msg.channel_id.say(&ctx.http, s).await?;
                    return Ok(());
                }
            };
            choice as i64
        }
        _ => {
            msg.channel_id.say(&ctx.http, "I didn't recognize that setting you tried to change. Run `bb-settings options` to see usage").await;
            return Ok(());
        }
    };

    if dbcontext.set_attr(&guild_id, &*setting_name, dbval).await {
        msg.channel_id.say(&ctx.http, ":+1: Updated.").await?;
    } else {
        msg.channel_id
            .say(&ctx.http, "Problems arose when saving those changes to settings. If this persists, try `bb-settings reset`")
            .await?;
    }
    Ok(())
}

async fn get_roll_from_set_command(ctx: &Context, msg: &Message, choice: &String) -> Option<i64> {
    Some(
        (if let Some(rol) = get_roll_id_by_name(&ctx, &msg, &choice[..]).await {
            rol
        } else if let Some(rol) = get_roll_id_by_name(&ctx, &msg, &choice.to_lowercase()[..]).await
        {
            rol
        } else if let Some(rol) = id_from_mention(&choice[..]) {
            rol
        } else {
            let s = "Broski that didn't look like a roll that you have here :(";
            msg.channel_id.say(&ctx.http, s).await;
            return None;
        }) as i64,
    )
}

#[command]
#[required_permissions("ADMINISTRATOR")]
async fn show(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    // redundant perm check is neccissary cuz lib bypasses the perms when running a group default command
    let g = ctx
        .cache
        .guild(msg.guild_id.expect("wtf"))
        .await
        .expect("wtf bro");
    match g.member_permissions(&ctx.http, msg.author.id).await {
        Ok(p) => {
            if !p.contains(Permissions::ADMINISTRATOR) {
                return Ok(());
            }
        }
        Err(_) => return Ok(()),
    };

    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0,
        None => 0,
    };

    let settings = dbcontext.get_settings(&guild).await;
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
    {}.
    Ping spam limits are {}.

    **Blacklists:**
    Newly joining members with these usernames or avatars will be __{}__
    username, simple: __{} entries__
    username, regex: __{} entries__
    avatar: __{} entries__
    Run `bb-blacklist` to see them and learn how to change them.

    All underlined items are configurable - run `bb-settings options` for more about that.
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
        },
        if Action::Nothing == settings.mentionaction {
            String::from("__disabled__ - it isn't implemented yet")
        } else {
            format!("__enabled__:\n      - Members will be __{}__ if they ping __{}__ users, __{}__ mentions, or __{}__ of either within __{}__ seconds", match settings.mentionaction {
                Action::Ban => "banned",
                Action::Kick => "kicked",
                Action::Mute => "muted",
                Action::Nothing => "this state is not reachable in code",
            }, settings.usermentions, settings.rollmentions, settings.anymentions, settings.mentiontime)
        },
        match settings.blacklistaction {
            Action::Ban => "banned",
            Action::Kick => "kicked",
            Action::Mute => "muted",
            Action::Nothing => "left alone"
        },
        settings.blacklist.simplename.len(),
        settings.blacklist.regexname.len(),
        settings.blacklist.avatar.len()
    );

    msg.channel_id.say(&ctx.http, s).await?;
    Ok(())
}

/// Retrieves the nth `char` of a `&str`
/// If `n < 0` then it will always retrieve the final `char`
/// If you attempt to index out of bounds, it will return a ` `
fn nth_char(s: &str, n: isize) -> char {
    if n < 0 {
        return s.chars().last().unwrap_or(' ');
    };
    s.chars().nth(n.try_into().unwrap()).unwrap_or(' ')
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
            println!("guild is cached ");
            if let Some(role) = guild.role_by_name(name) {
                println!("yeeeeeeetus");
                return Some(role.id.0);
            }
        }
    }
    None
}

fn humanize_duration(mut secs: i64) -> String {
    println!("{}", secs);
    let s = secs % 60;
    secs -= s;
    let m = (secs % 3600) / 60;
    secs -= m * 60;
    let h = (secs % (3600 * 24)) / 3600;
    secs -= h * (3600);
    let d = (secs % (3600 * 24 * 365)) / (3600 * 24);
    secs -= d * (3600 * 24);
    let y = secs / (3600 * 24 * 365);
    format!("{}y {}d {}h {}m {}s", y, d, h, m, s)
}
