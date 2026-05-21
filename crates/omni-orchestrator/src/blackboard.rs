use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct Blackboard {
    conn: Arc<Mutex<Connection>>,
    project_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct BlackboardEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub written_by: String,
    pub updated_at: DateTime<Utc>,
}

impl Blackboard {
    pub fn open(path: &Path, project_id: Uuid) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS blackboard (
                project_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                written_by TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (project_id, key)
            );
            CREATE INDEX IF NOT EXISTS idx_bb_project ON blackboard(project_id);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            project_id,
        })
    }

    pub fn in_memory(project_id: Uuid) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS blackboard (
                project_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                written_by TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (project_id, key)
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            project_id,
        })
    }

    pub async fn set<V: Serialize>(&self, key: &str, value: &V, written_by: &str) -> Result<()> {
        let json = serde_json::to_string(value)?;
        let pid = self.project_id.to_string();
        let key = key.to_string();
        let written_by = written_by.to_string();
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO blackboard (project_id, key, value, written_by, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(project_id, key) DO UPDATE SET
                value = excluded.value,
                written_by = excluded.written_by,
                updated_at = excluded.updated_at",
            params![pid, key, json, written_by],
        )?;
        Ok(())
    }

    pub async fn get<V: DeserializeOwned>(&self, key: &str) -> Result<Option<V>> {
        let pid = self.project_id.to_string();
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT value FROM blackboard WHERE project_id = ?1 AND key = ?2",
        )?;
        let result: Option<String> = stmt
            .query_row(params![pid, key], |row| row.get(0))
            .ok();
        match result {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn get_raw(&self, key: &str) -> Result<Option<serde_json::Value>> {
        self.get(key).await
    }

    pub async fn delete(&self, key: &str) -> Result<bool> {
        let pid = self.project_id.to_string();
        let conn = self.conn.lock().await;
        let affected = conn.execute(
            "DELETE FROM blackboard WHERE project_id = ?1 AND key = ?2",
            params![pid, key],
        )?;
        Ok(affected > 0)
    }

    pub async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let pid = self.project_id.to_string();
        let pattern = format!("{}%", prefix);
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT key FROM blackboard WHERE project_id = ?1 AND key LIKE ?2 ORDER BY key",
        )?;
        let keys = stmt
            .query_map(params![pid, pattern], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(keys)
    }

    pub async fn get_entry(&self, key: &str) -> Result<Option<BlackboardEntry>> {
        let pid = self.project_id.to_string();
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT key, value, written_by, updated_at FROM blackboard WHERE project_id = ?1 AND key = ?2",
        )?;
        let entry = stmt
            .query_row(params![pid, key], |row| {
                Ok(BlackboardEntry {
                    key: row.get(0)?,
                    value: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    written_by: row.get(2)?,
                    updated_at: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .ok();
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_get_delete() {
        let bb = Blackboard::in_memory(Uuid::new_v4()).unwrap();
        bb.set("game_design", &serde_json::json!({"genre": "platformer"}), "coordinator")
            .await
            .unwrap();

        let val: Option<serde_json::Value> = bb.get("game_design").await.unwrap();
        assert_eq!(val.unwrap()["genre"], "platformer");

        bb.delete("game_design").await.unwrap();
        let val: Option<serde_json::Value> = bb.get("game_design").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn test_list_keys() {
        let bb = Blackboard::in_memory(Uuid::new_v4()).unwrap();
        bb.set("asset/sprite_player", &"done", "asset_agent").await.unwrap();
        bb.set("asset/sprite_enemy", &"done", "asset_agent").await.unwrap();
        bb.set("code/main_scene", &"done", "code_agent").await.unwrap();

        let keys = bb.list_keys("asset/").await.unwrap();
        assert_eq!(keys.len(), 2);
    }
}
