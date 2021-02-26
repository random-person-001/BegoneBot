use crate::autopanic::Gramma;
use crate::db::{Action, MyDbContext};
use crate::{autopanic, db, Settings};
use serenity::builder::CreateMessage;
use serenity::model::channel::Embed;
use serenity::model::guild::Guild;
use serenity::model::id::{GuildId, RoleId};
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
use serenity::model::guild::Region::UsEast;

#[command]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_count = ctx.cache.guild_count().await;
    msg.channel_id.send_message(&ctx, |m| m.embed(|e| {
        e.title("About");
        e.description(format!(
        "I'm a focused and powerful antiraid bot in rust written by John Locke#2742 serving {} guilds.

        I'm completely free and will never have any paywalled features. However, if you appreciate my service, you can consider donating in the support server to keep me running.

        Pro tip: instead of writing out settings or blacklist, you can shorten them to s and bl respectively.\
        ",
        guild_count));
        e.field("Links","[Support server](https://discord.gg/eGNDZGdtaR)  |  [invite me](https://discordapp.com/oauth2/authorize?client_id=802019556801511424&scope=bot&permissions=268716070)", false);
        e
    })).await;
    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await?;
    Ok(())
}

#[command]
async fn blacklist_show(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let data = ctx.data.write().await;
    let dbcontext = data
        .get::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let g = &ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();

    let settings = dbcontext.get_settings(&g.id.0).await.expect("hmm");

    if unauthorized(&ctx, settings, g, msg, PermissionLevel::CanPanic).await {
        return Ok(());
    }

    let msg = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Current Blacklist" );
                e.color(0x070707);
                e.description(" \
                 • To adjust what happens when a member joins who matches a blacklist item, use `bb-settings set blacklistaction` with `ban`, `kick`, `mute`, or `nothing` at the end (default is kick). \n \
                 • To blacklist anyone who has the same profile picture (avatar) as the user 802019556801511424, run `bb-blacklist add avatar 802019556801511424` \n \
                 • To blacklist any users who are named EvilBotnet, run `bb-blacklist add name EvilBotnet` \n \
                 • To blacklist anyone named like DMSpammer4 or DMSpammer87, run `bb-blacklist add regexname DMSpammer\\d+` \
                Regex is extremely powerful, and there are many resources on the web to help you, like [this](https://regex101.com/). \
                \n
                 • If you'd like to remove an item, `bb-blacklist remove` will be your friend. \n\
                 • If you'd like to remove the first entry of the regex name blacklist, run `bb-blacklist remove regexname 1` \
                \n
                When talking about these lists in commands (like those examples above), call them `name` `regexname` and `avatar` respectively. \
                ");
                e.field("Simple name blacklist",
                format!("(will match username exactly)\n`{:?}`", settings.blacklist.simplename),
                false);
                e.field("Regex name blacklist",
                format!("(will match username by regex)\n`{:?}`", settings.blacklist.regexname),
                false);
                e.field("Avatar hash blacklist",
                format!("(will match an avatar's hash, which is unique to a profile picture. Due to technical limitations, from here you can't see the pictures they're referring to, but you could try searching them in chat)\n`{:?}`", settings.blacklist.avatar),
                false);
                e
            });
            m
        })
        .await;
    Ok(())
}

#[command]
async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");

    let g = &ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();
    let settings = dbcontext.get_settings(&g.id.0).await.expect("hmm");

    if unauthorized(&ctx, settings, g, msg, PermissionLevel::CanChangeSettings).await {
        return Ok(());
    }

    let mut s = settings.blacklist.clone();

    if args.len() != 2 {
        println!("{:?}", args);
        msg.channel_id.say(&ctx.http, "I need exactly two things specified after this: which list you're adding to (one of `name`, `regexname`, or `avatar`), and the item to add.").await;
        return Ok(());
    }

    let which_list = args.single::<String>().unwrap();
    let new_item = args.single::<String>().unwrap();
    match &*which_list {
        "name" => s.simplename.push(new_item),
        "regexname" => s.regexname.push(new_item),
        "avatar" => {
            let uid: u64 = match new_item.parse() {
                Ok(s) => s,
                Err(_) => {
                    msg.channel_id.say(&ctx.http, "broski i needed a user id there. If you wanna blacklist the avatar that some user 4534 has, run `bb-blacklist add avatar 4534`").await;
                    return Ok(());
                }
            };
            match UserId(uid).to_user(&ctx).await {
                Ok(u) => {
                    if u.avatar.is_some() {
                        s.avatar.push(u.avatar.clone().unwrap());
                        msg.channel_id.say(&ctx, format!("The user {0} - {1}#{2} has an avatar hash of {3}\nhttps://cdn.discordapp.com/avatars/{0}/{3}",
                        u.id, u.name, u.discriminator, u.avatar.unwrap())).await;
                    } else {
                        msg.channel_id
                            .say(
                                &ctx,
                                "Bro they have the default avatar so imma say nope to that",
                            )
                            .await;
                        return Ok(());
                    }
                }
                Err(_) => {
                    msg.channel_id.say(&ctx.http, format!("Hey uh so I couldn't find any existing user with the id of {} so I couldn't add their avatar to the blacklist", uid)).await;
                    return Ok(());
                }
            }
        }
        _ => {
            msg.channel_id.say(&ctx.http, "Broski....... please specify the blacklist you're trying to add to here. It can be one of `name` `regexname` `avatar`").await;
            return Ok(());
        }
    };
    if dbcontext.save_blacklist(&g.id.0, s).await {
        msg.channel_id.say(&ctx.http, ":+1: Added").await;
    } else {
        msg.channel_id
            .say(&ctx.http, "There was a problem saving that setting")
            .await;
    }
    Ok(())
}

#[command]
/// usage: bb-blacklist remove nameregex 4 to run settings.blacklist[3].pop()
async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");

    let g = &ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();
    let settings = dbcontext.get_settings(&g.id.0).await.expect("hmm");
    let mut s = settings.blacklist.clone();

    if unauthorized(&ctx, settings, g, msg, PermissionLevel::CanChangeSettings).await {
        return Ok(());
    }

    if args.len() != 2 {
        println!("{:?}", args);
        msg.channel_id.say(&ctx.http, "I need exactly two things specified after this: which list you're removing from (`avatar`, `name`, or `regexname`), and the index of the item you want to remove.").await;
        return Ok(());
    }

    let which_list: String = args.single::<String>().unwrap();

    let index = match args.single::<usize>() {
        Ok(i) => i,
        Err(_) => {
            msg.channel_id.say(&ctx.http, "Oops I need the _index_ of the item you're removing").await;
            return Ok(());
        }
    };

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
                removed = s.simplename.remove(index - 1);
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
                removed = s.regexname.remove(index - 1);
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
                removed = s.avatar.remove(index - 1);
            }
        }
        _ => {
            msg.channel_id.say(&ctx.http, "That option was not recognized. Acceptable inputs are `name` `regexname` `avatar`").await;
            return Ok(());
        }
    };
    if dbcontext.save_blacklist(&g.id.0, s).await {
        msg.channel_id
            .say(&ctx.http, format!("Successfully removed `{}`", removed))
            .await;
    } else {
        msg.channel_id.say(&ctx.http, "Problems occurred while updating db :_(\n if they persist, tell the dev or try `db-settings reset`").await;
    };
    Ok(())
}

#[command]
async fn reset(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");

    let g = &ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();
    let settings = dbcontext.get_settings(&g.id.0).await.expect("hmm");

    if unauthorized(&ctx, settings, g, msg, PermissionLevel::CanChangeSettings).await {
        return Ok(());
    }

    if dbcontext.add_guild(&g.id.0).await {
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
async fn forceban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();
    let i_can_ban = match guild.member_permissions(&ctx, msg.author.id).await {
        Ok(p) => p.contains(Permissions::BAN_MEMBERS),
        Err(_) => false,
    };
    let they_can_ban = match guild.member_permissions(&ctx, UserId(802019556801511424)).await {
        Ok(p) => p.contains(Permissions::BAN_MEMBERS),
        Err(_) => false,
    };
    if !they_can_ban {
        msg.channel_id.say(&ctx, "You don't have perms to ban members on your own, so imma nope you there").await;
        return Ok(())
    }
    if !i_can_ban {
        msg.channel_id.say(&ctx, "bro....... I don't have ban perms. What did you expect me to do").await;
        return Ok(())
    }

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
                        e.color(0x65ba7a);
                        e.description(format!(
                            "{}\nAccount created on {}\nwhich is {} ago",
                            u.mention(),
                            account_creation,
                            age
                        ));
                        e.thumbnail(&u.face());
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
async fn panic(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let settings = {
        let data = ctx.data.write().await;
        let dbcontext = data
            .get::<MyDbContext>()
            .expect("Expected MyDbContext in TypeMap.");

        let g = &ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();
        let guild_id = g.id.0;
        let settings = dbcontext.get_settings(&g.id.0).await.expect("hmm").clone();

        if unauthorized(&ctx, &settings, g, msg, PermissionLevel::CanPanic).await {
            return Ok(());
        }
        settings
    };
    let guild_id = msg.guild_id.unwrap().0;
    let mut data2 = ctx.data.write().await;
    let mut mom = data2
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
        autopanic::start_panicking(&ctx, mom, &settings, guild_id, true).await;
        return Ok(());
    }

    let choice = args.clone().single::<String>().unwrap().to_lowercase();
    println!("'{}'", choice);
    match &choice[..] {
        "on" => {
            println!("Panic mode manually activated for server {}", guild_id);
            mom.panicking = true;
            autopanic::start_panicking(&ctx, mom, &settings, guild_id, true).await;
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
        }
    }

    Ok(())
}

async fn options(ctx: &Context, msg: &Message) -> CommandResult {
    let s = "    _Example:_ To set the raid panic to trigger when 3 people join in 4 seconds, send these two messages:
`bb-settings set users 3`
`bb-settings set time 4`

_Example:_ To ping a roll named @Staff when a raid starts (assumes log channel has been set):
`bb-settings set notify Staff`

The following are the settings you can adjust:";
    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.title("Configuring Settings");
            e.color(0x607277);
            e.description(s);
            e.field("enabled", "whether automatic raid detection and prevention should occur. Input is either `true` or `false`", false);
            e.field("action", "What to do to to new joining members during a raid. One of `ban` `kick` `mute` `nothing`", false);
            e.field("users", "How many users it takes to trigger a raid panic. A number", false);
            e.field("time", "When `users` join in this amount of seconds, a raid panic is triggered. A number", false);
            e.field("logs", "What channel to post raid notifications. Is none by default. A blue channel name", false);
            e.field("notify", "When a raid panic starts, this roll is pinged in the `logs` channel.  A roll id or mention or written name", false);
            e.field("roll_that_can_change_settings", "Anyone with this roll or who is above it can change settings and alter the blacklist. A roll mention or name", false);
            e.field("roll_that_can_panic", "Anyone with this roll or higher can manually turn panic mode on and off, as well as view all settings. A roll mention or name", false);
            e.field("muteroll", "If an action is to apply a mute to someone upon joining, this is that mute roll.  A roll mention or name", false);
            e.field("blacklistaction", "When a new member joins who matches the blacklist, this will be done to them. One of `ban` `kick` `mute` `nothing`", false);
            e.field(".", "Adding and removing from blacklists can be done with `bb-blacklist`", false);
            e.footer(|f| {
                f.text("To check current settings, run just bb-settings")
            });
            e
        })
    }).await;
    Ok(())
}

#[command]
async fn help(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    msg.channel_id.send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("BegoneBot Help");
                e.color(0xc72a69);
                e.description(" \
                I'm a small but powerful bot that helps protect servers against raids.  \
                ");
                e.field("panic", "Go into panic mode to counter a raid. To turn off, run _bb-panic off_", false);
                e.field("settings", "View the current configuration settings. Pretty useful!", true);
                e.field("settings set", "See what settings you can adjust, or adjust them", true);
                e.field("settings reset", "Reset every setting to defaults", true);
                e.field("forceban", "Ban multiple users by their ids, separated by spaces. Banning users who aren't here works too", false);
                e.field("blacklist", "Show the blacklists that apply to new members joining, and how to add to them", true);
                e.field("blacklist add", "Add an entry to the blacklist", true);
                e.field("blacklist remove", "Remove an entry from the blacklist by index", true);
                e.field("uinfo", "View profile pic and creation date of any account, not necessarily a member here, by id.", false);
                e.field("about", "Learn a little more about me", false);
                e.footer(|f| {
                    f.text("my prefix is bb-");
                    f.icon_url("https://cdn.discordapp.com/avatars/802019556801511424/b8a65c225df2567ddb95cf5158fdeab8.webp?size=128");
                    f
                });
                e
            })}).await;
    Ok(())
}

#[command]
async fn set(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.is_empty() {
        options(&ctx, &msg).await;
        return Ok(());
    }
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");

    let g = &ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();
    let settings = dbcontext.get_settings(&g.id.0).await.expect("hmm");

    println!("authorizing");
    if unauthorized(&ctx, settings, g, msg, PermissionLevel::CanChangeSettings).await {
        println!("unauthorized");
        return Ok(());
    }
    println!("authorized");

    let guild_id = msg.guild_id.expect("").0;
    let setting_name = args.single::<String>().unwrap().to_lowercase();

    if args.is_empty() {
        msg.channel_id.say(
            &ctx.http,
            "I need a value to set that to. Run `bb-settings set` for more",
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
                    if let Some(rol) =
                        get_roll_id_by_name_case_insensitive(&ctx, &msg, "muted").await
                    {
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
                    let s = "Bro that didn't look like a normal channel. Make sure it turns blue and clickable";
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
        "roll_that_can_panic" => {
            if let Some(h) = get_roll_from_set_command(&ctx, &msg, &choice).await {
                h
            } else {
                return Ok(());
            }
        }
        "roll_that_can_change_settings" => {
            if let Some(h) = get_roll_from_set_command(&ctx, &msg, &choice).await {
                h
            } else {
                return Ok(());
            }
        }
        _ => {
            msg.channel_id.say(&ctx.http, "I didn't recognize that setting you tried to change. Run `bb-settings set` to see usage").await;
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

async fn get_roll_from_set_command(ctx: &Context, msg: &Message, choice: &str) -> Option<i64> {
    Some(
        (if let Some(rol) = get_roll_id_by_name(&ctx, &msg, &choice[..]).await {
            rol
        } else if let Some(rol) =
            get_roll_id_by_name_case_insensitive(&ctx, &msg, &choice.to_lowercase()[..]).await
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
async fn show(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    // redundant perm check is necessary cuz lib bypasses the perms when running a group default command
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");

    let g = &ctx.cache.guild(msg.guild_id.unwrap()).await.unwrap();
    let settings = dbcontext.get_settings(&g.id.0).await.expect("hmm");

    if unauthorized(&ctx, settings, g, msg, PermissionLevel::CanPanic).await {
        return Ok(());
    }

    if !args.is_empty() {
        msg.channel_id.say(&ctx, "Hey looks like you put something after your command. Did you remember to include `set` after `bb-settings`?").await;
    }

    msg.channel_id.send_message(&ctx.http, |m| m.embed(|e| {
        e.title("Settings");
        e.color(0x4d8290);
        e.description("All underlined items are configurable - run `bb-settings set` for more about that.");
        e.field("Automatic Antiraid", format!(
            "Automatic raid detection is currently __{}__
            Panic mode is automatically triggered when __{}__ users join in __{}__ seconds.
            During panic mode:
            _ _  - server verification level will be turned to Highest (verified phone required to join)
            _ _  - any member joining will be {}",

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
        ),
            false);
        e.field("General", format!("\
            {}.\n\
            Ping spam limits are {}.\n\
            {} can view settings and toggle panic mode.\n\
            {} can change settings and alter the blacklist",

        if let 0 = settings.logs {
            String::from("__No__ logging channel is configured")
        } else {
            format!(
                "Logs are posted in __<#{}>__ \n    __{}__ is pinged when a raid is detected\n",
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
            if settings.roll_that_can_panic > 0 {
                format!("__<@&{}>__ and higher", settings.roll_that_can_panic)
            } else {
                String::from("__Server admins__")
            },
            if settings.roll_that_can_change_settings > 0 {
                format!("__<@&{}>__ and higher", settings.roll_that_can_change_settings)
            } else {
                String::from("__Server admins__")
            },
        ),
        false);
        e.field("Blacklists", format!("
    Newly joining members with these usernames or avatars will be __{}__
    _ _  - username, simple: __{} entries__
    _ _  - username, regex: __{} entries__
    _ _  - avatar: __{} entries__
    Run `bb-blacklist` to see them and learn how to change them.
",
        match settings.blacklistaction {
            Action::Ban => "banned",
            Action::Kick => "kicked",
            Action::Mute => "muted",
            Action::Nothing => "left alone",
        },
        settings.blacklist.simplename.len(),
        settings.blacklist.regexname.len(),
        settings.blacklist.avatar.len()
        ), false);
        e
    })).await;

    // embeds -
    // https://github.com/serenity-rs/serenity/blob/current/examples/e09_create_message_builder/src/main.rs
    Ok(())
}

enum PermissionLevel {
    CanPanic,
    CanChangeSettings,
}

// return whether someone is authorized to run a command or not.  Will send a helpful message if they aren't
async fn unauthorized(
    ctx: &Context,
    settings: &db::Settings,
    guild: &Guild,
    msg: &Message,
    perm_level: PermissionLevel,
) -> bool {
    let author = msg.author.id;
    let critical_roll = &match perm_level {
        PermissionLevel::CanPanic => RoleId(settings.roll_that_can_panic),
        PermissionLevel::CanChangeSettings => RoleId(settings.roll_that_can_change_settings),
    };
    let is_admin = match guild.member_permissions(&ctx, author).await {
        Ok(p) => p.contains(Permissions::ADMINISTRATOR),
        Err(_) => false,
    };

    return if settings.roll_that_can_panic == 0 || !guild.roles.contains_key(critical_roll) {
        match is_admin {
            true => false,
            false => {
                msg.channel_id.say(&ctx, format!("Since nothing is set for my `{0}` setting, I require you to have admin perms here to run that command. Run `bb-settings set {0} Staff` for example to let the roll named @Staff run this command", match perm_level {
                    PermissionLevel::CanPanic => "roll_that_can_panic",
                    PermissionLevel::CanChangeSettings => "roll_that_can_change_settings"
                })).await;
                true
            }
        }
    } else if guild
        .members
        .get(&author)
        .unwrap()
        .roles
        .contains(critical_roll)
        || is_admin
    {
        false
    } else {
        let roll_name = &guild.roles.get(critical_roll).unwrap().name;
        msg.channel_id.say(&ctx, format!("Hey my settings here require you to have the {} roll to run that.  A high ranking member can change that by running `bb-settings set {} Staff` for instance to let anyone with the Staff roll use this",
                                         roll_name,
                                         match perm_level {
                                             PermissionLevel::CanPanic => "roll_that_can_panic",
                                             PermissionLevel::CanChangeSettings => "roll_that_can_change_settings"
                                         },
        )).await;
        true
    };
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

/// Find a roll id such that roll.lowercase() == msg (assumed to be lowercase)
async fn get_roll_id_by_name_case_insensitive(
    ctx: &Context,
    msg: &Message,
    name: &str,
) -> Option<u64> {
    if let Some(guild_id) = msg.guild_id {
        if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
            if let Some(role) = guild
                .roles
                .values()
                .find(|role| name == role.name.clone().to_lowercase())
            {
                println!("yeeeeeeetus");
                return Some(role.id.0);
            }
        }
    }
    None
}

/// Find a roll id from a roll that matches name and case exactly
async fn get_roll_id_by_name(ctx: &Context, msg: &Message, name: &str) -> Option<u64> {
    if let Some(guild_id) = msg.guild_id {
        if let Some(guild) = guild_id.to_guild_cached(&ctx).await {
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
    let hour = (secs % (3600 * 24)) / 3600;
    secs -= hour * (3600);
    let day = (secs % (3600 * 24 * 365)) / (3600 * 24);
    secs -= day * (3600 * 24);
    let year = secs / (3600 * 24 * 365);
    format!("{}y {}d {}h {}m {}s", year, day, hour, m, s)
}
