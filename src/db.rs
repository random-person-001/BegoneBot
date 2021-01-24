
use rusqlite::{Connection, Statement, NO_PARAMS};


/// What to do to noobs when shit hits the fan (autopanic on)
#[derive(Debug)]
pub enum Action {
    Ban,
    Kick,
    Mute,
}

/// Per-guild settings (aka a full row in the sql table, but with dif types)
#[derive(Debug)]
pub struct Settings {
    guild: u32,
    enabled: u8,
    action: Action,
    users: u32,
    time: i32,
    logs: u32,
}


#[derive(Debug)]
pub struct MyDbContext<'a> {
    pub conn: &'a Connection,
    pub add_guild_table_statement: Option<Statement<'a>>,
    pub add_guild_statement: Option<Statement<'a>>,
    pub fetch_settings_statement: Option<Statement<'a>>,
    pub get_settings_statement: Option<Statement<'a>>,
    pub add_enabled_statement: Option<Statement<'a>>,
    pub add_users_statement: Option<Statement<'a>>,
    pub add_time_statement: Option<Statement<'a>>,
    pub add_action_statement: Option<Statement<'a>>,
}

impl<'a> MyDbContext<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        return MyDbContext {
            conn,
            add_guild_statement: None,
            add_guild_table_statement: None,
            fetch_settings_statement: None,
            get_settings_statement: None,
            add_enabled_statement: None,
            add_users_statement: None,
            add_time_statement: None,
            add_action_statement: None,
        };
    }

    pub fn add_guild_table(&mut self) {
        if let None = &self.add_guild_table_statement {
            let stmt = self.conn.prepare(
                "
                    CREATE TABLE IF NOT EXISTS settings (
                    guild INTEGER PRIMARY KEY,
                    enabled INTEGER DEFAULT 0,
                    action INTEGER DEFAULT 1,
                    users INTEGER DEFAULT 5,
                    time INTEGER DEFAULT 25,
                    logs INTEGER DEFAULT 0);
                ",
            );
            self.add_guild_table_statement = Some(stmt.unwrap());
        }
        match self.add_guild_table_statement
            .as_mut()
            .unwrap()
            .execute(NO_PARAMS) {
            Ok(_) => println!("Yeet guild table should exist now"),
            Err(_) => println!("Yo an add guild table call failed - guild"),
        };
    }

    pub fn add_guild(&mut self, guild_id: i32) {
        self.add_guild_table();
        if let None = &self.add_guild_statement {
            let stmt = self
                .conn
                .prepare("INSERT INTO settings (guild) VALUES (?1);");
            self.add_guild_statement = Some(stmt.unwrap());
        }
        match self.add_guild_statement
            .as_mut()
            .unwrap()
            .execute(&[&guild_id]) {
            Ok(_) => println!("Yo made an entry for guild {}", guild_id),
            Err(_) => println!("Yo a get guild call failed - guild {}", guild_id),
        };
    }

    pub fn fetch_settings(&mut self, guild_id: i32) -> Option<Settings> {
        if let None = &self.get_settings_statement {
            let stmt = self.conn.prepare("SELECT * FROM settings WHERE guild = ?;");
            self.get_settings_statement = Some(stmt.unwrap());
        }

        let settings =
            self.get_settings_statement
                .as_mut()
                .unwrap()
                .query_map(&[&guild_id], |row| {
                    Ok(Settings {
                        guild: row.get(0)?,
                        enabled: row.get(1)?,
                        action: match row.get(2)? {
                            0 => Action::Ban,
                            1 => Action::Kick,
                            2 => Action::Mute,
                            _default => Action::Kick,
                        },
                        users: row.get(3)?,
                        time: row.get(4)?,
                        logs: row.get(5)?,
                    })
                });
        for i in settings.unwrap() {
            return Some(i.unwrap());
        }
        None
    }

    pub fn set_enabled(&mut self, guild: i32, enabled: bool) {
        if let None = &self.add_enabled_statement {
            let stmt = self
                .conn
                .prepare("UPDATE settings SET enabled = ?1 WHERE guild = ?2");
            self.add_enabled_statement = Some(stmt.unwrap());
        }
        let enabled = match enabled {
            true => 1,
            false => 0,
        };
        match self.add_enabled_statement
            .as_mut()
            .unwrap()
            .execute(&[&enabled, &guild]){
            Ok(_) => (),
            Err(_) => println!("Yo a Set Enabled call failed - guild {}", guild),
        };
    }

    pub fn set_users(&mut self, guild: i32, users: i32) {
        if let None = &self.add_users_statement {
            let stmt = self
                .conn
                .prepare("UPDATE settings SET users = ?1 WHERE guild = ?2");
            self.add_users_statement = Some(stmt.unwrap());
        }
        match self.add_users_statement
            .as_mut()
            .unwrap()
            .execute(&[&users, &guild]){
            Ok(_) => (),
            Err(_) => println!("Yo a Set Users call failed - guild {}", guild),
        };
    }

    pub fn set_time(&mut self, guild: i32, time: i32) {
        if let None = &self.add_time_statement {
            let stmt = self
                .conn
                .prepare("UPDATE settings SET time = ?1 WHERE guild = ?2");
            self.add_time_statement = Some(stmt.unwrap());
        }
        match self.add_time_statement
            .as_mut()
            .unwrap()
            .execute(&[&time, &guild]){
            Ok(_) => (),
            Err(_) => println!("Yo a Set Time call failed - guild {}", guild),
        };
    }

    pub fn set_action(&mut self, guild: i32, action: Action) {
        if let None = &self.add_action_statement {
            let stmt = self
                .conn
                .prepare("UPDATE settings SET action = ?1 WHERE guild = ?2");
            self.add_action_statement = Some(stmt.unwrap());
        }
        let action = match action {
            Action::Ban => 0,
            Action::Kick => 1,
            Action::Mute => 2,
        };
        match self.add_action_statement
            .as_mut()
            .unwrap()
            .execute(&[&action, &guild]) {
            Ok(_) => (),
            Err(_) => println!("Yo a Set Action call failed - guild {}", guild),
        };
    }
}
