use serenity::model::application::MembershipState::Accepted;
use sqlx::database::HasValueRef;
use sqlx::{query_as_with, sqlx_macros, Database, Decode, Encode, Result, Sqlite};
use std::collections::HashMap;
use std::convert::TryInto;
use std::error::Error;
use serde::Serialize;
use serde::Deserialize;
use crate::blob_blacklist_conversions::{decode, encode};

/// What to do to noobs when shit hits the fan (autopanic on)
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Action {
    Ban,
    Kick,
    Mute,
    Nothing,
}

impl From<Action> for i64 {
    fn from(a: Action) -> Self {
        match a {
            Action::Ban => 0,
            Action::Kick => 1,
            Action::Mute => 2,
            Action::Nothing => 3,
        }
    }
}

impl From<i64> for Action {
    fn from(n: i64) -> Self {
        match n {
            0 => Action::Ban,
            1 => Action::Kick,
            2 => Action::Mute,
            3 => Action::Nothing,
            _ => Action::Kick,
        }
    }
}

/// todo: store this in the db later
#[derive(Debug, Clone, sqlx::FromRow, sqlx::Encode, sqlx::Decode, sqlx::Type, serde::Deserialize, serde::Serialize)]
pub struct Blacklist {
    pub simplename: Vec<String>,
    pub regexname: Vec<String>,
    pub avatar: Vec<String>,
}

impl Blacklist {
    fn new() -> Self{
        Blacklist {
            simplename: vec![],
            regexname: vec![],
            avatar: vec![],
        }
    }
    fn from_bytes(raw: Vec<u8>) -> Self {
        let mut stuffs = decode(raw);
        //println!("from_bytes-> {:#?}", stuffs);
        assert!(stuffs.len() >= 3);
        // maybe using serde instead of custom serialization would eliminate the need for these hacks
        if stuffs[0][0].is_empty() {
            stuffs[0].remove(0);
        }
        if stuffs[1][0].is_empty() {
            stuffs[1].remove(0);
        }
        if stuffs[2][0].is_empty() {
            stuffs[2].remove(0);
        }
        Blacklist {
            simplename: stuffs[0].clone(),
            regexname: stuffs[1].clone(),
            avatar: stuffs[2].clone(),
        }
    }
    fn to_bytes(&self) -> Vec<u8> {
        encode(vec![&self.simplename, &self.regexname, &self.avatar])
    }
}


/// Per-guild settings (aka a full row in the sql table, but with dif types)
#[derive(Debug,  Clone)]
pub struct Settings {
    pub guild: u64,
    pub enabled: bool,
    pub action: Action,
    pub users: u8,
    pub time: u32,
    pub logs: u64,
    pub muteroll: u64,
    pub rollmentions: u8,
    pub usermentions: u8,
    pub anymentions: u8,
    pub mentionaction: Action,
    pub mentiontime: u32,
    pub notify: u64, // who to ping in logs when stuff goes down
    pub blacklist: Blacklist,
    pub blacklistaction: Action,
}

#[derive(Debug, sqlx::FromRow)]
struct RawSettings {
    guild: u64,
    enabled: i32,
    action: i64,
    users: i32,
    time: i32,
    logs: u64,
    muteroll: u64,
    rollmentions: i32,
    usermentions: i32,
    anymentions: i32,
    mentionaction: i64,
    mentiontime: i32,
    notify: u64,
    blacklist: Vec<u8>,
    blacklistaction: i64
}

impl From<RawSettings> for Settings {
    fn from(s: RawSettings) -> Self {
        Settings {
            guild: s.guild,
            enabled: s.enabled != 0,
            action: s.action.try_into().unwrap(),
            users: s.users.try_into().unwrap(),
            time: s.time.try_into().unwrap(),
            logs: s.logs,
            muteroll: s.muteroll,
            rollmentions: s.rollmentions.try_into().unwrap(),
            usermentions: s.usermentions.try_into().unwrap(),
            anymentions: s.anymentions.try_into().unwrap(),
            mentionaction: s.mentionaction.try_into().unwrap(),
            mentiontime: s.mentiontime.try_into().unwrap(),
            notify: s.notify,
            blacklist: Blacklist::from_bytes(s.blacklist),
            blacklistaction: s.blacklistaction.try_into().unwrap()
        }
    }
}

#[derive(Debug)]
pub struct MyDbContext {
    pool: sqlx::SqlitePool,
    pub cache: HashMap<u64, Settings>,
}

impl MyDbContext {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        MyDbContext {
            pool,
            cache: HashMap::new(),
        }
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
                muteroll INTEGER DEFAULT 0,
                rollmentions INTEGER DEFAULT 4,
                usermentions INTEGER DEFAULT 6,
                anymentions INTEGER DEFAULT 8,
                mentionaction INTEGER DEFAULT 3,
                mentiontime INTEGER DEFAULT 5,
                notify INTEGER DEFAULT 0,
                blacklist BLOB DEFAULT NULL,
                blacklistaction INTEGER DEFAULT 1
                );
            ";
        match sqlx::query(q).execute(&self.pool).await {
            Ok(_) => true,
            Err(why) => {
                println!("Encountered error creating settings db: {:?}", why);
                false
            }
        }
    }

    pub async fn add_guild(&mut self, guild: &u64) -> bool {
        self.add_guild_table().await;
        sqlx::query("DELETE FROM settings WHERE guild=?1;")
            .bind(&guild)
            .execute(&self.pool)
            .await.is_ok();
        match sqlx::query("INSERT INTO settings (guild, blacklist) VALUES (?1, ?2);")
            .bind(&guild)
            .bind(Blacklist::new().to_bytes())
            .execute(&self.pool)
            .await
        {
            Ok(_) => {
                let settings = self.fetch_settings(&guild).await.unwrap();
                self.cache.insert(*guild, settings);
                true
            }
            Err(why) => {
                println!("Something went wrong adding guild: {:?}", why);
                false
            }
        }
    }

    pub async fn get_settings(&self, guild_id: &u64) -> Option<&Settings> {
        self.cache.get(guild_id)
    }

    pub async fn fetch_settings(&mut self, guild_id: &u64) -> Option<Settings> {
        let r: Result<RawSettings> = sqlx::query_as("SELECT * FROM settings WHERE guild = ?;")
            .bind(guild_id)
            .fetch_one(&self.pool)
            .await;
        let s = match r {
            Ok(raw_settings) => Some(raw_settings),
            Err(msg) => {
                println!("Something went wrong getting settings: {:?}", msg);
                None
            }
        };
        println!("{:?}", s);
        match s {
            Some(s) => Some(s.try_into().unwrap()),
            None => None,
        }
    }

    pub async fn set_enabled(&mut self, guild: &u64, enabled: bool) -> bool {
        match sqlx::query("UPDATE settings SET enabled = ?1 WHERE guild = ?2")
            .bind(enabled as i32)
            .bind(&guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => {
                self.cache.get_mut(&guild).unwrap().enabled = enabled;
                true
            }
            Err(why) => {
                println!(
                    "Something went wrong setting enabled of {}: {:?}",
                    guild, why
                );
                false
            }
        }
    }

    pub async fn save_blacklist(&mut self, guild: &u64, new_bl: Blacklist) -> bool {
        match sqlx::query("UPDATE settings SET blacklist = ?1 WHERE guild = ?2")
            .bind(new_bl.to_bytes())
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => {
                self.cache.get_mut(guild).expect("yeetus").blacklist = new_bl.clone();
                true
            },
            Err(why) => {
                println!("Saving blacklist failed: {}", why);
                false
            }

        }
    }

    pub async fn set_attr(&mut self, guild: &u64, key: &str, value: i64) -> bool {
        // format macro because sql sucks sometimes ig
        let s = &*format!("UPDATE settings SET {} = ?1 WHERE guild = ?2", key);
        //println!("{}", s);
        match sqlx::query(s)
            .bind(value)
            .bind(guild)
            .execute(&self.pool)
            .await
        {
            Ok(_) => {
                let mut settings = self.cache.get_mut(guild).unwrap();
                match key {
                    "users" => settings.users = value.try_into().unwrap(),
                    "time" => settings.time = value.try_into().unwrap(),
                    "action" => settings.action = value.try_into().unwrap(),
                    "logs" => settings.logs = value.try_into().unwrap(),
                    "muteroll" => settings.muteroll = value.try_into().unwrap(),
                    "rollmentions" => settings.rollmentions = value.try_into().unwrap(),
                    "usermentions" => settings.usermentions = value.try_into().unwrap(),
                    "anymentions" => settings.anymentions = value.try_into().unwrap(),
                    "mentiontime" => settings.mentiontime = value.try_into().unwrap(),
                    "mentionaction" => settings.mentionaction = value.try_into().unwrap(),
                    "notify" => settings.notify = value.try_into().unwrap(),
                    "blacklistaction" => settings.blacklistaction = value.try_into().unwrap(),
                    s => {
                        println!(
                            "Broski I couldn't update the cached settings cuz {} wasn't in there",
                            s
                        );
                        return false;
                    }
                };
                true
            }
            Err(why) => {
                println!("Error updating settings db for {}: {:?}", guild, why);
                false
            }
        }
    }
}
