/// ATP database (as distinct from blockstore)
use crate::{AtpSession, KeyPair};
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

    /// Quick check if an account already exists for given username or email
    pub fn account_exists(&mut self, username: &str, email: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT COUNT(*) FROM account WHERE username = $1 OR email = $2")?;
        let count: i32 = stmt.query_row(params!(username, email), |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn create_account(
        &mut self,
        did: &str,
        username: &str,
        password: &str,
        email: &str,
        recovery_pubkey: &str,
    ) -> Result<()> {
        debug!("bcrypt hashing password (can be slow)...");
        let password_bcrypt = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO account (username, password_bcrypt, email, did, recovery_pubkey) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        stmt.execute(params!(
            username,
            password_bcrypt,
            email,
            did,
            recovery_pubkey
        ))?;
        Ok(())
    }

    /// Returns a JWT session token
    pub fn create_session(
        &mut self,
        username: &str,
        password: &str,
        keypair: &KeyPair,
    ) -> Result<AtpSession> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT did, password_bcrypt FROM account WHERE username = ?1")?;
        let (did, password_bcrypt): (String, String) =
            stmt.query_row(params!(username), |row| Ok((row.get(0)?, row.get(1)?)))?;
        if !bcrypt::verify(password, &password_bcrypt)? {
            return Err(anyhow!("password did not match"));
        }
        let jwt = keypair.ucan()?;
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO session (did, jwt) VALUES (?1, ?2)")?;
        stmt.execute(params!(did, jwt))?;
        Ok(AtpSession {
            did,
            name: username.to_string(),
            accessJwt: jwt.to_string(),
            refreshJwt: jwt.to_string(),
        })
    }

    /// Returns the DID that a token is valid for
    pub fn check_auth_token(&mut self, jwt: &str) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT did FROM session WHERE jwt = $1")?;
        let did = stmt.query_row(params!(jwt), |row| row.get(0))?;
        Ok(did)
    }

    pub fn put_did_doc(&mut self, did: &str, did_doc: &Value) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO did_doc (did, doc_json) VALUES (?1, ?2)")?;
        stmt.execute(params!(did, did_doc.to_string()))?;
        Ok(())
    }
}
