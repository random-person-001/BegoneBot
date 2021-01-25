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
pub struct MyDbContext {
    pub conn: Connection,
}

impl MyDbContext {
    pub fn new(conn: Connection) -> Self {
        conn.set_prepared_statement_cache_capacity(20);
        return MyDbContext { conn };
    }

    pub fn add_guild_table(&mut self) {
        let mut stmt = self.conn.prepare_cached(
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
        stmt.execute(NO_PARAMS);
    }

    pub fn add_guild(&mut self, guild_id: i32) {
        self.add_guild_table();
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO settings (guild) VALUES (?1);");
        stmt.execute(&[&guild_id]);
    }

    pub fn fetch_settings(&mut self, guild_id: i32) -> Option<Settings> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM settings WHERE guild = ?;");

        let settings = stmt.query_map(&[&guild_id], |row| {
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
        let mut stmt = self
            .conn
            .prepare_cached("UPDATE settings SET enabled = ?1 WHERE guild = ?2");
        let enabled = match enabled {
            true => 1,
            false => 0,
        };
        match stmt.execute(&[&enabled, &guild]) {
            Ok(_) => (),
            Err(_) => println!("Yo a Set Enabled call failed - guild {}", guild),
        };
    }

    pub fn set_users(&mut self, guild: i32, users: i32) {
        let mut stmt = self
            .conn
            .prepare_cached("UPDATE settings SET users = ?1 WHERE guild = ?2");
        match stmt.execute(&[&users, &guild]) {
            Ok(_) => (),
            Err(_) => println!("Yo a Set Users call failed - guild {}", guild),
        };
    }

    pub fn set_time(&mut self, guild: i32, time: i32) {
        let mut stmt = self
            .conn
            .prepare_cached("UPDATE settings SET time = ?1 WHERE guild = ?2");
        match stmt.execute(&[&time, &guild]) {
            Ok(_) => (),
            Err(_) => println!("Yo a Set Time call failed - guild {}", guild),
        };
    }

    pub fn set_action(&mut self, guild: i32, action: Action) {
        let mut stmt = self
            .conn
            .prepare_cached("UPDATE settings SET action = ?1 WHERE guild = ?2");
        let action = match action {
            Action::Ban => 0,
            Action::Kick => 1,
            Action::Mute => 2,
        };
        match stmt.execute(&[&action, &guild]) {
            Ok(_) => (),
            Err(_) => println!("Yo a Set Action call failed - guild {}", guild),
        };
    }
}
