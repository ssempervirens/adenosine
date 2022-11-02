use crate::AtpSession;
/// ATP database (as distinct from blockstore)
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use log::debug;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_test() {
        assert!(MIGRATIONS.validate().is_ok());
    }
}

lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![M::up(include_str!("atp_db.sql")),]);
}

#[derive(Debug)]
pub struct AtpDatabase {
    conn: Connection,
}

impl AtpDatabase {
    pub fn open(path: &PathBuf) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        MIGRATIONS.to_latest(&mut conn)?;
        // any pragma would happen here
        Ok(AtpDatabase { conn })
    }

    /// temporary database, eg for tests.
    ///
    /// TODO: should create a tmp file on ramdisk (/var/tmp?) instead of opening an in-memory
    /// database. in-memory database can't be used with multiple connections
    pub fn open_ephemeral() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        MIGRATIONS.to_latest(&mut conn)?;
        // any pragma would happen here
        Ok(AtpDatabase { conn })
    }

    /// Creates an entirely new connection to the same database
    ///
    /// Skips re-running migrations.
    ///
    /// Fails for ephemeral databases.
    pub fn new_connection(&self) -> Result<Self> {
        // TODO: let path = std::path::PathBuf::from(self.conn.path().ok_or(Err(anyhow!("expected real database")))?);
        let path = std::path::PathBuf::from(self.conn.path().expect("expected real database"));
        let conn = Connection::open(path)?;
        Ok(AtpDatabase { conn })
    }

    pub fn get_record(&mut self, did: &str, collection: &str, tid: &str) -> Result<Value> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT record_json FROM record WHERE did = ?1 AND collection = ?2 AND tid = ?3",
        )?;
        Ok(stmt.query_row(params!(did, collection, tid), |row| {
            row.get(0).map(|v: String| Value::from_str(&v))
        })??)
    }

    pub fn get_record_list(&mut self, did: &str, collection: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT tid FROM record WHERE did = ?1 AND collection = ?2")?;
        let ret = stmt
            .query_and_then(params!(did, collection), |row| {
                let v: String = row.get(0)?;
                Ok(v)
            })?
            .collect();
        ret
    }

    pub fn get_collection_list(&mut self, did: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT collection FROM record WHERE did = ?1 GROUP BY collection")?;
        let ret = stmt
            .query_and_then(params!(did), |row| {
                let v: String = row.get(0)?;
                Ok(v)
            })?
            .collect();
        ret
    }

    pub fn create_account(
        &mut self,
        username: &str,
        password: &str,
        email: &str,
    ) -> Result<AtpSession> {
        // TODO: validate email (regex?)
        // TODO: validate username
        // TODO: generate and store signing key
        // TODO: generate plc did (randomly for now?)
        // TODO: insert did_doc
        // TODO: also need to initialize repo with... profile?
        {
            debug!("bcrypt hashing password (can be slow)...");
            let password_bcrypt = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
            let signing_key = "key:TODO";
            let did = "did:TODO";
            let mut stmt = self
                .conn
                .prepare_cached("INSERT INTO account (username, password_bcrypt, email, did, signing_key) VALUES (?1, ?2, ?3, ?4, ?5)")?;
            stmt.execute(params!(username, password_bcrypt, email, did, signing_key))?;
        }
        self.create_session(username, password)
    }

    pub fn create_session(&mut self, username: &str, password: &str) -> Result<AtpSession> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT password_bcrypt FROM account WHERE username = ?1")?;
        let password_bcrypt: String = stmt.query_row(params!(username), |row| row.get(0))?;
        if !bcrypt::verify(password, &password_bcrypt)? {
            return Err(anyhow!("password did not match"));
        }
        // TODO: generate JWT
        // TODO: insert session wtih JWT
        Ok(AtpSession {
            name: username.to_string(),
            did: "did:TODO".to_string(),
            jwt: "jwt:TODO".to_string(),
        })
    }
}
