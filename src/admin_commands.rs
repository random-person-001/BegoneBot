use crate::db::MyDbContext;
use serenity::{framework::standard::{macros::command, Args, CommandResult}, model::prelude::Message, prelude::Context, http};
use std::process::exit;
use std::process::Command;
use crate::{Settings, autopanic, garbage_collect};
use crate::autopanic::{time_now, YourMama};
use std::collections::HashMap;
use std::borrow::Cow;

#[command]
async fn garbage(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    msg.channel_id.say(&ctx, "Collecting garbage...").await;
    garbage_collect(&ctx);
    msg.channel_id.say(&ctx, "Done").await;
    Ok(())
}

#[command]
async fn delete(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    if msg.author.id.0 != 275384719024193538 {
        return Ok(());
    }
    let mut data = ctx.data.write().await;
    let mut dbcontext = data
        .get_mut::<MyDbContext>()
        .expect("Expected MyDbContext in TypeMap.");
    let guild: u64 = match msg.guild_id {
        Some(id) => id.0,
        None => 0,
    };
    if dbcontext.drop_guild(&guild).await {
        msg.channel_id.say(&ctx, "yeet").await;
    } else {
        msg.channel_id.say(&ctx, "sad").await;
    }
    Ok(())
}


#[command]
async fn foo(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let bad_guys: Vec<u64> = vec![234,465346,46346346,234563463,23459845093840,94235029834509,98324,203948,70934,293485,93245];
    let string = bad_guys.iter().fold(String::new(), |to_return, id| format!("{} {}", to_return, id));

    let attachment = http::AttachmentType::Bytes {
        filename: String::from("Raiders.txt"),
        data: Cow::from(string.into_bytes())
    };
    msg.channel_id.send_files(&ctx, vec![attachment], |m| {
        m.content("here's some files bruh")
    }).await?;
    Ok(())
}

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
async fn free(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    if msg.author.id.0 != 275384719024193538 {
        return Ok(());
    }
    msg.channel_id.say(&ctx, format!("```\n{}\n```", free_mem().await)).await;
    Ok(())
}

#[command]
async fn git_push(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    if msg.author.id.0 != 275384719024193538 {
        return Ok(());
    }
    msg.channel_id.say(&ctx, "Working....").await;

    if !push().await {
        msg.channel_id.say(&ctx, "Git push failed.").await;
        return Ok(());
    }
    msg.channel_id.say(&ctx, "Git push success.").await;
    Ok(())
}

#[command]
async fn update(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    if msg.author.id.0 != 275384719024193538 {
        return Ok(());
    }
    msg.channel_id.say(&ctx, "Working....").await;

    if !pull().await {
        msg.channel_id.say(&ctx, "Git pull failed.").await;
        return Ok(());
    }

    // we retry compilation several times, because sometimes the droplet isn't strong and
    // intermittently fails at it
    let mut n: i32 = 4;
    loop {
        n -= 1;
        if !build_release().await {
            msg.channel_id.say(&ctx,format!( "build release failed - {:} tries left", n)).await;
            if n < 0 {
                msg.channel_id.say(&ctx, "Build release failed.").await;
                break;
            }
        } else {
            msg.channel_id
            .say(&ctx, "Pulled from github and successfully built")
            .await;
            break;
        }
    }
    Ok(())
}

async fn push() -> bool {
    let output = Command::new("git").arg("push").output();
    return output.is_ok() && output.unwrap().status.success();
}

async fn pull() -> bool {
    let output = Command::new("git").arg("pull").output();
    return output.is_ok() && output.unwrap().status.success();
}

async fn build_release() -> bool {
    let output = Command::new("cargo").arg("build").arg("--release").output();
    if output.is_ok() {
        let new = output.unwrap();
        println!("{}", String::from_utf8(new.stdout).unwrap());
        return new.status.success();
    }
    return false;
}

async fn free_mem() -> String {
    let output = Command::new("free").arg("-h").output();
    if output.is_ok() {
        let new = output.unwrap();
        String::from_utf8(new.stdout).unwrap()
    } else {
        println!("{:?}", output.err().unwrap());
        String::from("Error with command")
    }
}
