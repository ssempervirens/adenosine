/// ATP database (as distinct from blockstore)
use crate::{AtpSession, Did, KeyPair};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use log::debug;
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;

/// Default is 12, but that is quite slow (on my laptop at least)
const BCRYPT_COST: u32 = 8;

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

    /// Quick check if an account already exists for given handle or email
    pub fn account_exists(&mut self, handle: &str, email: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT COUNT(*) FROM account WHERE handle = $1 OR email = $2")?;
        let count: i32 = stmt.query_row(params!(handle, email), |row| row.get(0))?;
        Ok(count > 0)
    }

    pub fn create_account(
        &mut self,
        did: &Did,
        handle: &str,
        password: &str,
        email: &str,
        recovery_pubkey: &str,
    ) -> Result<()> {
        debug!("bcrypt hashing password (can be slow)...");
        let password_bcrypt = bcrypt::hash(password, BCRYPT_COST)?;
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO account (handle, password_bcrypt, email, did, recovery_pubkey) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        stmt.execute(params!(
            handle,
            password_bcrypt,
            email,
            did.to_string(),
            recovery_pubkey,
        ))?;
        Ok(())
    }

    /// Returns a JWT session token
    pub fn create_session(
        &mut self,
        handle: &str,
        password: &str,
        keypair: &KeyPair,
    ) -> Result<AtpSession> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT did, password_bcrypt FROM account WHERE handle = ?1")?;
        let (did_col, password_bcrypt): (String, String) =
            stmt.query_row(params!(handle), |row| Ok((row.get(0)?, row.get(1)?)))?;
        if !bcrypt::verify(password, &password_bcrypt)? {
            return Err(anyhow!("password did not match"));
        }
        let did = Did::from_str(&did_col)?;
        let jwt = keypair.ucan(&did)?;
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO session (did, jwt) VALUES (?1, ?2)")?;
        stmt.execute(params!(did.to_string(), jwt))?;
        Ok(AtpSession {
            did: did.to_string(),
            name: handle.to_string(),
            accessJwt: jwt.to_string(),
            refreshJwt: jwt,
        })
    }

    /// Returns the DID that a token is valid for, or None if session not found
    pub fn check_auth_token(&mut self, jwt: &str) -> Result<Option<Did>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT did FROM session WHERE jwt = $1")?;
        let did_maybe: Option<String> =
            stmt.query_row(params!(jwt), |row| row.get(0)).optional()?;
        Ok(did_maybe.map(|v| Did::from_str(&v).expect("valid DID in database")))
    }

    pub fn delete_session(&mut self, jwt: &str) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare_cached("DELETE FROM session WHERE jwt = $1")?;
        let count = stmt.execute(params!(jwt))?;
        Ok(count >= 1)
    }

    pub fn put_did_doc(&mut self, did: &Did, did_doc: &Value) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT INTO did_doc (did, doc_json) VALUES (?1, ?2)")?;
        stmt.execute(params!(did.to_string(), did_doc.to_string()))?;
        Ok(())
    }
    pub fn get_did_doc(&mut self, did: &Did) -> Result<Value> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT doc_json FROM did_doc WHERE did = $1")?;
        let doc_json: String = stmt.query_row(params!(did.to_string()), |row| row.get(0))?;
        Ok(Value::from_str(&doc_json)?)
    }
}
