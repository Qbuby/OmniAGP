use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GddDocument {
    pub id: Uuid,
    pub project_id: Uuid,
    pub mechanics: Vec<Mechanic>,
    pub entities: Vec<Entity>,
    pub levels: Vec<Level>,
    pub metadata: GddMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GddMetadata {
    pub title: String,
    pub genre: String,
    pub target_platform: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mechanic {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub name: String,
    pub entity_type: String,
    pub properties: serde_json::Value,
    pub behaviors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub layout: serde_json::Value,
    pub entities: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GddVersion {
    pub id: Uuid,
    pub gdd_id: Uuid,
    pub version_number: u64,
    pub snapshot: GddDocument,
    pub author_id: Uuid,
    pub message: String,
    pub parent_version: Option<Uuid>,
    pub branch_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GddBranch {
    pub name: String,
    pub gdd_id: Uuid,
    pub head_version: Uuid,
    pub created_from: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffEntry {
    MechanicAdded(Mechanic),
    MechanicRemoved(Uuid),
    MechanicModified { id: Uuid, field: String, old: serde_json::Value, new: serde_json::Value },
    EntityAdded(Entity),
    EntityRemoved(Uuid),
    EntityModified { id: Uuid, field: String, old: serde_json::Value, new: serde_json::Value },
    LevelAdded(Level),
    LevelRemoved(Uuid),
    LevelModified { id: Uuid, field: String, old: serde_json::Value, new: serde_json::Value },
    MetadataChanged { field: String, old: String, new: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GddDiff {
    pub from_version: Uuid,
    pub to_version: Uuid,
    pub entries: Vec<DiffEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtOperation {
    pub id: Uuid,
    pub author_id: Uuid,
    pub gdd_id: Uuid,
    pub op_type: CrdtOpType,
    pub path: String,
    pub value: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub clock: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrdtOpType {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub user_id: Uuid,
    pub section: String,
    pub field_path: String,
    pub updated_at: DateTime<Utc>,
}

pub struct VersionStore {
    versions: HashMap<Uuid, GddVersion>,
    branches: HashMap<(Uuid, String), GddBranch>,
    documents: HashMap<Uuid, GddDocument>,
    version_counter: HashMap<Uuid, u64>,
}

impl VersionStore {
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
            branches: HashMap::new(),
            documents: HashMap::new(),
            version_counter: HashMap::new(),
        }
    }

    pub fn create_document(&mut self, project_id: Uuid, metadata: GddMetadata) -> GddDocument {
        let doc = GddDocument {
            id: Uuid::new_v4(),
            project_id,
            mechanics: Vec::new(),
            entities: Vec::new(),
            levels: Vec::new(),
            metadata,
        };
        self.documents.insert(doc.id, doc.clone());
        self.version_counter.insert(doc.id, 0);
        doc
    }

    pub fn save_version(
        &mut self,
        gdd_id: Uuid,
        author_id: Uuid,
        message: String,
        branch_name: &str,
    ) -> Option<GddVersion> {
        let doc = self.documents.get(&gdd_id)?.clone();
        let counter = self.version_counter.get_mut(&gdd_id)?;
        *counter += 1;

        let parent = self
            .branches
            .get(&(gdd_id, branch_name.to_string()))
            .map(|b| b.head_version);

        let version = GddVersion {
            id: Uuid::new_v4(),
            gdd_id,
            version_number: *counter,
            snapshot: doc,
            author_id,
            message,
            parent_version: parent,
            branch_name: branch_name.to_string(),
            created_at: Utc::now(),
        };

        let branch = GddBranch {
            name: branch_name.to_string(),
            gdd_id,
            head_version: version.id,
            created_from: parent.unwrap_or(version.id),
            created_at: Utc::now(),
        };

        self.versions.insert(version.id, version.clone());
        self.branches.insert((gdd_id, branch_name.to_string()), branch);
        Some(version)
    }

    pub fn get_version(&self, version_id: &Uuid) -> Option<&GddVersion> {
        self.versions.get(version_id)
    }

    pub fn get_document(&self, gdd_id: &Uuid) -> Option<&GddDocument> {
        self.documents.get(gdd_id)
    }

    pub fn get_document_mut(&mut self, gdd_id: &Uuid) -> Option<&mut GddDocument> {
        self.documents.get_mut(gdd_id)
    }

    pub fn rollback(&mut self, version_id: &Uuid) -> Option<&GddDocument> {
        let version = self.versions.get(version_id)?.clone();
        self.documents.insert(version.gdd_id, version.snapshot);
        self.documents.get(&version.gdd_id)
    }

    pub fn create_branch(&mut self, gdd_id: Uuid, branch_name: &str, from_version: Uuid) -> Option<GddBranch> {
        if !self.versions.contains_key(&from_version) {
            return None;
        }
        let branch = GddBranch {
            name: branch_name.to_string(),
            gdd_id,
            head_version: from_version,
            created_from: from_version,
            created_at: Utc::now(),
        };
        self.branches.insert((gdd_id, branch_name.to_string()), branch.clone());
        Some(branch)
    }

    pub fn list_versions(&self, gdd_id: Uuid, branch_name: &str) -> Vec<&GddVersion> {
        let mut versions: Vec<&GddVersion> = self
            .versions
            .values()
            .filter(|v| v.gdd_id == gdd_id && v.branch_name == branch_name)
            .collect();
        versions.sort_by_key(|v| v.version_number);
        versions
    }

    pub fn diff(&self, from_id: &Uuid, to_id: &Uuid) -> Option<GddDiff> {
        let from = self.versions.get(from_id)?;
        let to = self.versions.get(to_id)?;

        let mut entries = Vec::new();

        for mechanic in &to.snapshot.mechanics {
            if !from.snapshot.mechanics.iter().any(|m| m.id == mechanic.id) {
                entries.push(DiffEntry::MechanicAdded(mechanic.clone()));
            }
        }
        for mechanic in &from.snapshot.mechanics {
            if !to.snapshot.mechanics.iter().any(|m| m.id == mechanic.id) {
                entries.push(DiffEntry::MechanicRemoved(mechanic.id));
            }
        }

        for entity in &to.snapshot.entities {
            if !from.snapshot.entities.iter().any(|e| e.id == entity.id) {
                entries.push(DiffEntry::EntityAdded(entity.clone()));
            }
        }
        for entity in &from.snapshot.entities {
            if !to.snapshot.entities.iter().any(|e| e.id == entity.id) {
                entries.push(DiffEntry::EntityRemoved(entity.id));
            }
        }

        for level in &to.snapshot.levels {
            if !from.snapshot.levels.iter().any(|l| l.id == level.id) {
                entries.push(DiffEntry::LevelAdded(level.clone()));
            }
        }
        for level in &from.snapshot.levels {
            if !to.snapshot.levels.iter().any(|l| l.id == level.id) {
                entries.push(DiffEntry::LevelRemoved(level.id));
            }
        }

        if from.snapshot.metadata.title != to.snapshot.metadata.title {
            entries.push(DiffEntry::MetadataChanged {
                field: "title".into(),
                old: from.snapshot.metadata.title.clone(),
                new: to.snapshot.metadata.title.clone(),
            });
        }

        Some(GddDiff {
            from_version: *from_id,
            to_version: *to_id,
            entries,
        })
    }
}

impl Default for VersionStore {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CollaborationSession {
    pub gdd_id: Uuid,
    pub participants: HashMap<Uuid, CursorPosition>,
    pub pending_ops: Vec<CrdtOperation>,
    pub op_clock: u64,
}

impl CollaborationSession {
    pub fn new(gdd_id: Uuid) -> Self {
        Self {
            gdd_id,
            participants: HashMap::new(),
            pending_ops: Vec::new(),
            op_clock: 0,
        }
    }

    pub fn join(&mut self, user_id: Uuid) {
        self.participants.insert(
            user_id,
            CursorPosition {
                user_id,
                section: String::new(),
                field_path: String::new(),
                updated_at: Utc::now(),
            },
        );
    }

    pub fn leave(&mut self, user_id: &Uuid) {
        self.participants.remove(user_id);
    }

    pub fn update_cursor(&mut self, user_id: Uuid, section: String, field_path: String) {
        if let Some(cursor) = self.participants.get_mut(&user_id) {
            cursor.section = section;
            cursor.field_path = field_path;
            cursor.updated_at = Utc::now();
        }
    }

    pub fn apply_operation(&mut self, mut op: CrdtOperation) -> CrdtOperation {
        self.op_clock += 1;
        op.clock = self.op_clock;
        op.timestamp = Utc::now();
        self.pending_ops.push(op.clone());
        op
    }

    pub fn get_cursors(&self) -> Vec<&CursorPosition> {
        self.participants.values().collect()
    }

    pub fn flush_ops(&mut self) -> Vec<CrdtOperation> {
        std::mem::take(&mut self.pending_ops)
    }
}
