use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub const BCRYPT_COST: u32 = 12;
pub const MIN_PASSWORD_LEN: usize = 8;

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct UserStore {
    conn: Arc<Mutex<Connection>>,
}

impl UserStore {
    pub fn open(path: &Path) -> Result<Self, UserError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self, UserError> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), UserError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);",
        )?;
        Ok(())
    }

    pub async fn create_user(&self, username: &str, password: &str) -> Result<User, UserError> {
        let username = username.trim();
        if username.is_empty() || username.len() > 64 {
            return Err(UserError::InvalidUsername);
        }
        if password.len() < MIN_PASSWORD_LEN {
            return Err(UserError::PasswordTooShort);
        }
        let hash = bcrypt::hash(password, BCRYPT_COST)
            .map_err(|e| UserError::Internal(e.to_string()))?;
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let conn = self.conn.lock().await;
        let res = conn.execute(
            "INSERT INTO users (id, username, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, username, hash, created_at.to_rfc3339()],
        );
        match res {
            Ok(_) => Ok(User {
                id,
                username: username.to_string(),
                created_at,
            }),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(UserError::UsernameTaken)
            }
            Err(e) => Err(UserError::Internal(e.to_string())),
        }
    }

    pub async fn verify_credentials(
        &self,
        username: &str,
        password: &str,
    ) -> Result<User, UserError> {
        let row: Option<(String, String, String, String)> = {
            let conn = self.conn.lock().await;
            conn.query_row(
                "SELECT id, username, password_hash, created_at FROM users WHERE username = ?1",
                params![username],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?
        };
        let Some((id, username, hash, created_at)) = row else {
            return Err(UserError::InvalidCredentials);
        };
        let ok = bcrypt::verify(password, &hash)
            .map_err(|e| UserError::Internal(e.to_string()))?;
        if !ok {
            return Err(UserError::InvalidCredentials);
        }
        Ok(User {
            id,
            username,
            created_at: parse_rfc3339_or_now(&created_at),
        })
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<User>, UserError> {
        let conn = self.conn.lock().await;
        let row: Option<(String, String, String)> = conn
            .query_row(
                "SELECT id, username, created_at FROM users WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        Ok(row.map(|(id, username, created_at)| User {
            id,
            username,
            created_at: parse_rfc3339_or_now(&created_at),
        }))
    }
}

fn parse_rfc3339_or_now(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[derive(Debug)]
pub enum UserError {
    UsernameTaken,
    InvalidCredentials,
    PasswordTooShort,
    InvalidUsername,
    Internal(String),
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserError::UsernameTaken => write!(f, "username already taken"),
            UserError::InvalidCredentials => write!(f, "invalid credentials"),
            UserError::PasswordTooShort => write!(f, "password too short"),
            UserError::InvalidUsername => write!(f, "invalid username"),
            UserError::Internal(e) => write!(f, "internal error: {e}"),
        }
    }
}

impl std::error::Error for UserError {}

impl From<rusqlite::Error> for UserError {
    fn from(e: rusqlite::Error) -> Self {
        UserError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_user_then_verify_succeeds() {
        let store = UserStore::in_memory().expect("in-memory store");
        let user = store
            .create_user("alice", "password123")
            .await
            .expect("create user");
        assert_eq!(user.username, "alice");
        let back = store
            .verify_credentials("alice", "password123")
            .await
            .expect("verify creds");
        assert_eq!(back.id, user.id);
    }

    #[tokio::test]
    async fn duplicate_username_rejected() {
        let store = UserStore::in_memory().unwrap();
        store.create_user("bob", "password123").await.unwrap();
        let err = store
            .create_user("bob", "anotherpassword")
            .await
            .expect_err("should fail");
        assert!(matches!(err, UserError::UsernameTaken));
    }

    #[tokio::test]
    async fn password_below_minimum_rejected() {
        let store = UserStore::in_memory().unwrap();
        let err = store
            .create_user("carol", "short")
            .await
            .expect_err("should fail");
        assert!(matches!(err, UserError::PasswordTooShort));
    }

    #[tokio::test]
    async fn blank_or_overlong_username_rejected() {
        let store = UserStore::in_memory().unwrap();
        let blank = store
            .create_user("   ", "password123")
            .await
            .expect_err("blank should fail");
        assert!(matches!(blank, UserError::InvalidUsername));
        let overlong = "x".repeat(65);
        let too_long = store
            .create_user(&overlong, "password123")
            .await
            .expect_err("overlong should fail");
        assert!(matches!(too_long, UserError::InvalidUsername));
    }

    #[tokio::test]
    async fn wrong_password_rejected() {
        let store = UserStore::in_memory().unwrap();
        store.create_user("dave", "password123").await.unwrap();
        let err = store
            .verify_credentials("dave", "wrongpassword")
            .await
            .expect_err("should fail");
        assert!(matches!(err, UserError::InvalidCredentials));
    }

    #[tokio::test]
    async fn unknown_user_rejected() {
        let store = UserStore::in_memory().unwrap();
        let err = store
            .verify_credentials("ghost", "password123")
            .await
            .expect_err("should fail");
        assert!(matches!(err, UserError::InvalidCredentials));
    }

    #[tokio::test]
    async fn find_by_id_roundtrip() {
        let store = UserStore::in_memory().unwrap();
        let user = store.create_user("erin", "password123").await.unwrap();
        let found = store.find_by_id(&user.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "erin");
        let missing = store.find_by_id("nope").await.unwrap();
        assert!(missing.is_none());
    }
}
