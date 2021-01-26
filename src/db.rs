use serenity::model::application::MembershipState::Accepted;
use sqlx;
use sqlx::{query_as_with, sqlx_macros, Result, Sqlite};
use std::convert::TryInto;

/// What to do to noobs when shit hits the fan (autopanic on)
#[derive(Debug, PartialEq)]
pub enum Action {
    Ban,
    Kick,
    Mute,
}

/// Per-guild settings (aka a full row in the sql table, but with dif types)
pub struct Settings {
    pub guild: u64,
    pub enabled: u8,
    pub action: Action,
    pub users: u8,
    pub time: u32,
    pub logs: u64,
    pub muteroll: u64,
}

#[derive(Debug, sqlx::FromRow)]
struct RawSettings {
    guild: u64,
    enabled: i32,
    action: i32,
    users: i32,
    time: i32,
    logs: u64,
    muteroll: u64,
}

impl RawSettings {
    fn betterify(&self) -> Settings {
        let enum_action = match &self.action {
            0 => Action::Ban,
            1 => Action::Kick,
            2 => Action::Mute,
            _default => Action::Kick,
        };
        Settings {
            guild: self.guild.try_into().unwrap(),
            enabled: self.enabled.try_into().unwrap(),
            action: enum_action,
            users: self.users.try_into().unwrap(),
            time: self.time.try_into().unwrap(),
            logs: self.logs.try_into().unwrap(),
            muteroll: self.muteroll.try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct MyDbContext {
    pub pool: sqlx::SqlitePool,
}

impl MyDbContext {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        return MyDbContext { pool };
    }

    pub async fn add_guild_table(&self) -> bool {
        let q = "
                CREATE TABLE IF NOT EXISTS settings (
                guild INTEGER PRIMARY KEY,
                enabled INTEGER DEFAULT 0,
                action INTEGER DEFAULT 1,
                users INTEGER DEFAULT 5,
                time INTEGER DEFAULT 25,
                logs INTEGER DEFAULT 0,
                muteroll INTEGER DEFAULT 0);
            ";
        match sqlx::query(q).execute(&self.pool).await {
            Ok(_) => true,
            Err(why) => {
                println!("Encountered error creating settings db: {:?}", why);
                false
            }
        }
    }

    pub async fn add_guild(&mut self, guild_id: u64) -> bool {
        self.add_guild_table().await;
        match sqlx::query("DELETE FROM settings WHERE guild=?1;")
            .bind(guild_id)
            .execute(&self.pool)
            .await
        {
            Ok(_) => (),
            Err(_) => (),
        }
        match sqlx::query("INSERT INTO settings (guild) VALUES (?1);")
            .bind(guild_id)
            .execute(&self.pool)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn fetch_settings(&mut self, guild_id: &u64) -> Option<Settings> {
        let r: Result<RawSettings> = sqlx::query_as("SELECT * FROM settings WHERE guild = ?;")
            .bind(guild_id)
            .fetch_one(&self.pool)
            .await;
        let s = match r {
            Ok(raw_settings) => Some(raw_settings),
            Err(msg) => {
                println!("Something went wrong getting settings:");
                None
            }
        };
        println!("{:?}", s);
        match s {
            Some(s) => Some(s.betterify()),
            None => None,
        }
    }

    pub async fn set_enabled(&mut self, guild: u64, enabled: bool) -> bool {
        let enabled: i32 = match enabled {
            true => 1,
            false => 0,
        };
        match sqlx::query("UPDATE settings SET enabled = ?1 WHERE guild = ?2")
            .bind(enabled)
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn set_users(&mut self, guild: u64, users: u8) -> bool {
        match sqlx::query("UPDATE settings SET users = ?1 WHERE guild = ?2")
            .bind(users)
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn set_time(&mut self, guild: u64, time: u32) -> bool {
        match sqlx::query("UPDATE settings SET time = ?1 WHERE guild = ?2")
            .bind(time)
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn set_muteroll(&mut self, guild: u64, muteroll: u64) -> bool {
        match sqlx::query("UPDATE settings SET muteroll = ?1 WHERE guild = ?2")
            .bind(muteroll)
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn set_logs(&mut self, guild: u64, logs: u64) -> bool {
        match sqlx::query("UPDATE settings SET logs = ?1 WHERE guild = ?2")
            .bind(logs)
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn set_action(&mut self, guild: u64, action: &Action) -> bool {
        let action: i32 = match action {
            Action::Ban => 0,
            Action::Kick => 1,
            Action::Mute => 2,
        };
        match sqlx::query("UPDATE settings SET action = ?1 WHERE guild = ?2")
            .bind(action)
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
