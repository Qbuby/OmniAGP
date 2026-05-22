use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Role {
    Guest,
    Viewer,
    Editor,
    Owner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    ViewProject,
    EditGdd,
    GenerateGame,
    ManageMembers,
    DeleteProject,
    PublishGame,
    ManageWorkflow,
    ViewAuditLog,
}

impl Role {
    pub fn permissions(&self) -> &'static [Permission] {
        match self {
            Role::Guest => &[Permission::ViewProject],
            Role::Viewer => &[Permission::ViewProject, Permission::ViewAuditLog],
            Role::Editor => &[
                Permission::ViewProject,
                Permission::EditGdd,
                Permission::GenerateGame,
                Permission::ManageWorkflow,
                Permission::ViewAuditLog,
            ],
            Role::Owner => &[
                Permission::ViewProject,
                Permission::EditGdd,
                Permission::GenerateGame,
                Permission::ManageMembers,
                Permission::DeleteProject,
                Permission::PublishGame,
                Permission::ManageWorkflow,
                Permission::ViewAuditLog,
            ],
        }
    }

    pub fn has_permission(&self, perm: Permission) -> bool {
        self.permissions().contains(&perm)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMember {
    pub user_id: Uuid,
    pub project_id: Uuid,
    pub role: Role,
    pub invited_by: Uuid,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub action: AuditAction,
    pub details: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    ProjectCreated,
    MemberInvited,
    MemberRemoved,
    RoleChanged,
    GddEdited,
    VersionCreated,
    ReviewRequested,
    ReviewApproved,
    ReviewRejected,
    GameGenerated,
    GamePublished,
    WorkflowTransition,
}

#[derive(Debug, Clone)]
pub struct AccessControl {
    members: HashMap<(Uuid, Uuid), ProjectMember>,
    audit_log: Vec<AuditEntry>,
}

impl AccessControl {
    pub fn new() -> Self {
        Self {
            members: HashMap::new(),
            audit_log: Vec::new(),
        }
    }

    pub fn add_member(&mut self, member: ProjectMember) {
        let key = (member.user_id, member.project_id);
        self.record_audit(
            member.project_id,
            member.invited_by,
            AuditAction::MemberInvited,
            serde_json::json!({
                "user_id": member.user_id,
                "role": member.role,
            }),
        );
        self.members.insert(key, member);
    }

    pub fn remove_member(&mut self, user_id: Uuid, project_id: Uuid, removed_by: Uuid) {
        let key = (user_id, project_id);
        if self.members.remove(&key).is_some() {
            self.record_audit(
                project_id,
                removed_by,
                AuditAction::MemberRemoved,
                serde_json::json!({ "user_id": user_id }),
            );
        }
    }

    pub fn set_role(
        &mut self,
        user_id: Uuid,
        project_id: Uuid,
        new_role: Role,
        changed_by: Uuid,
    ) -> bool {
        let key = (user_id, project_id);
        if let Some(member) = self.members.get_mut(&key) {
            let old_role = member.role;
            member.role = new_role;
            self.record_audit(
                project_id,
                changed_by,
                AuditAction::RoleChanged,
                serde_json::json!({
                    "user_id": user_id,
                    "old_role": old_role,
                    "new_role": new_role,
                }),
            );
            true
        } else {
            false
        }
    }

    pub fn get_role(&self, user_id: Uuid, project_id: Uuid) -> Option<Role> {
        self.members
            .get(&(user_id, project_id))
            .map(|m| m.role)
    }

    pub fn check_permission(
        &self,
        user_id: Uuid,
        project_id: Uuid,
        permission: Permission,
    ) -> bool {
        self.get_role(user_id, project_id)
            .map(|role| role.has_permission(permission))
            .unwrap_or(false)
    }

    pub fn get_project_members(&self, project_id: Uuid) -> Vec<&ProjectMember> {
        self.members
            .values()
            .filter(|m| m.project_id == project_id)
            .collect()
    }

    pub fn record_audit(
        &mut self,
        project_id: Uuid,
        user_id: Uuid,
        action: AuditAction,
        details: serde_json::Value,
    ) {
        self.audit_log.push(AuditEntry {
            id: Uuid::new_v4(),
            project_id,
            user_id,
            action,
            details,
            timestamp: Utc::now(),
        });
    }

    pub fn get_audit_log(&self, project_id: Uuid) -> Vec<&AuditEntry> {
        self.audit_log
            .iter()
            .filter(|e| e.project_id == project_id)
            .collect()
    }
}

impl Default for AccessControl {
    fn default() -> Self {
        Self::new()
    }
}
