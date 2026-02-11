// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright (C) 2026 Relational Network

//! Audit logging for security-sensitive operations.
//!
//! All wallet operations, authentication events, and administrative
//! actions are logged to the encrypted audit store.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{EncryptedStorage, StorageResult};

/// Types of auditable events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Wallet events
    WalletCreated,
    WalletDeleted,
    WalletAccessed,

    // Transaction events
    TransactionSigned,
    TransactionBroadcast,

    // Bookmark events
    BookmarkCreated,
    BookmarkDeleted,

    // Invite events
    InviteCreated,
    InviteRedeemed,

    // Recurring payment events
    RecurringCreated,
    RecurringUpdated,
    RecurringDeleted,
    RecurringExecuted,

    // Auth events
    AuthSuccess,
    AuthFailure,
    PermissionDenied,

    // Admin events
    AdminAccess,
    ConfigChanged,
}

/// An audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditEvent {
    /// Unique event ID.
    pub event_id: String,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Type of event.
    pub event_type: AuditEventType,
    /// User who triggered the event (if known).
    pub user_id: Option<String>,
    /// Resource affected (wallet_id, bookmark_id, etc.).
    pub resource_id: Option<String>,
    /// Resource type (wallet, bookmark, etc.).
    pub resource_type: Option<String>,
    /// IP address of the request (if available).
    pub ip_address: Option<String>,
    /// Additional details as JSON.
    #[schema(value_type = Option<Object>)]
    pub details: Option<serde_json::Value>,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event.
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            user_id: None,
            resource_id: None,
            resource_type: None,
            ip_address: None,
            details: None,
            success: true,
            error: None,
        }
    }

    /// Set the user ID.
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the resource.
    pub fn with_resource(
        mut self,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
    ) -> Self {
        self.resource_type = Some(resource_type.into());
        self.resource_id = Some(resource_id.into());
        self
    }

    /// Set the IP address.
    /// TODO: Use when request IP tracking is implemented
    #[allow(dead_code)]
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Add details.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Mark as failed with error message.
    /// TODO: Use when error tracking for audited operations is implemented
    #[allow(dead_code)]
    pub fn failed(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error = Some(error.into());
        self
    }
}

/// Repository for audit events.
pub struct AuditRepository<'a> {
    storage: &'a EncryptedStorage,
}

impl<'a> AuditRepository<'a> {
    /// Create a new audit repository.
    pub fn new(storage: &'a EncryptedStorage) -> Self {
        Self { storage }
    }

    /// Log an audit event.
    ///
    /// Events are appended to a daily log file in JSONL format.
    pub fn log(&self, event: &AuditEvent) -> StorageResult<()> {
        let date = event.timestamp.format("%Y-%m-%d").to_string();
        let path = self.storage.paths().audit_events_file(&date);

        // Read existing events (or empty if file doesn't exist)
        let mut content = self.storage.read_raw(&path).unwrap_or_default();

        // Append new event as JSONL (one JSON object per line)
        let event_json = serde_json::to_string(event).map_err(|e| {
            super::StorageError::SerializationError(format!(
                "Failed to serialize audit event: {}",
                e
            ))
        })?;

        if !content.is_empty() && !content.ends_with(b"\n") {
            content.push(b'\n');
        }
        content.extend_from_slice(event_json.as_bytes());
        content.push(b'\n');

        self.storage.write_raw(&path, &content)
    }

    /// Read audit events for a specific date.
    pub fn read_events(&self, date: &str) -> StorageResult<Vec<AuditEvent>> {
        let path = self.storage.paths().audit_events_file(date);
        let content = self.storage.read_raw(&path)?;

        let content_str = String::from_utf8(content).map_err(|e| {
            super::StorageError::SerializationError(format!("Invalid UTF-8 in audit log: {}", e))
        })?;

        let mut events = Vec::new();
        for line in content_str.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: AuditEvent = serde_json::from_str(line).map_err(|e| {
                super::StorageError::SerializationError(format!(
                    "Failed to deserialize audit event: {}",
                    e
                ))
            })?;
            events.push(event);
        }

        Ok(events)
    }

    /// Read events for a date range.
    pub fn read_events_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> StorageResult<Vec<AuditEvent>> {
        use chrono::NaiveDate;

        let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").map_err(|e| {
            super::StorageError::SerializationError(format!("Invalid start date: {}", e))
        })?;

        let end = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").map_err(|e| {
            super::StorageError::SerializationError(format!("Invalid end date: {}", e))
        })?;

        let mut all_events = Vec::new();
        let mut current = start;

        while current <= end {
            let date_str = current.format("%Y-%m-%d").to_string();
            if let Ok(events) = self.read_events(&date_str) {
                all_events.extend(events);
            }
            current = current.succ_opt().ok_or_else(|| {
                super::StorageError::SerializationError("Date overflow".to_string())
            })?;
        }

        Ok(all_events)
    }

    /// Search events by user ID.
    /// TODO: Use when implementing user-specific audit views
    #[allow(dead_code)]
    pub fn search_by_user(&self, user_id: &str, date: &str) -> StorageResult<Vec<AuditEvent>> {
        let events = self.read_events(date)?;
        Ok(events
            .into_iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .collect())
    }

    /// Search events by resource.
    /// TODO: Use when implementing resource-specific audit views
    #[allow(dead_code)]
    pub fn search_by_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        date: &str,
    ) -> StorageResult<Vec<AuditEvent>> {
        let events = self.read_events(date)?;
        Ok(events
            .into_iter()
            .filter(|e| {
                e.resource_type.as_deref() == Some(resource_type)
                    && e.resource_id.as_deref() == Some(resource_id)
            })
            .collect())
    }
}

/// Helper macro for logging audit events.
#[macro_export]
macro_rules! audit_log {
    ($storage:expr, $event_type:expr, $user:expr) => {{
        let repo = $crate::storage::AuditRepository::new($storage);
        let event = $crate::storage::AuditEvent::new($event_type).with_user(&$user.user_id);
        let _ = repo.log(&event);
    }};
    ($storage:expr, $event_type:expr, $user:expr, $resource_type:expr, $resource_id:expr) => {{
        let repo = $crate::storage::AuditRepository::new($storage);
        let event = $crate::storage::AuditEvent::new($event_type)
            .with_user(&$user.user_id)
            .with_resource($resource_type, $resource_id);
        let _ = repo.log(&event);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{EncryptedStorage, StoragePaths};
    use tempfile::TempDir;

    fn setup() -> (TempDir, EncryptedStorage) {
        let temp = TempDir::new().unwrap();
        let paths = StoragePaths::new(temp.path().to_str().unwrap());
        let mut storage = EncryptedStorage::new(paths);
        storage.initialize().unwrap();
        (temp, storage)
    }

    #[test]
    fn create_audit_event() {
        let event = AuditEvent::new(AuditEventType::WalletCreated)
            .with_user("user_123")
            .with_resource("wallet", "wallet_abc")
            .with_ip("192.168.1.1");

        assert_eq!(event.event_type, AuditEventType::WalletCreated);
        assert_eq!(event.user_id, Some("user_123".to_string()));
        assert_eq!(event.resource_type, Some("wallet".to_string()));
        assert_eq!(event.resource_id, Some("wallet_abc".to_string()));
        assert!(event.success);
    }

    #[test]
    fn failed_event() {
        let event = AuditEvent::new(AuditEventType::PermissionDenied)
            .with_user("user_123")
            .failed("Not authorized");

        assert!(!event.success);
        assert_eq!(event.error, Some("Not authorized".to_string()));
    }

    #[test]
    fn log_and_read_events() {
        let (_temp, storage) = setup();
        let repo = AuditRepository::new(&storage);

        let event1 = AuditEvent::new(AuditEventType::WalletCreated)
            .with_user("user_1")
            .with_resource("wallet", "w1");

        let event2 = AuditEvent::new(AuditEventType::WalletAccessed)
            .with_user("user_2")
            .with_resource("wallet", "w2");

        repo.log(&event1).unwrap();
        repo.log(&event2).unwrap();

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let events = repo.read_events(&today).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, AuditEventType::WalletCreated);
        assert_eq!(events[1].event_type, AuditEventType::WalletAccessed);
    }

    #[test]
    fn search_by_user() {
        let (_temp, storage) = setup();
        let repo = AuditRepository::new(&storage);

        repo.log(
            &AuditEvent::new(AuditEventType::WalletCreated)
                .with_user("user_target")
                .with_resource("wallet", "w1"),
        )
        .unwrap();

        repo.log(
            &AuditEvent::new(AuditEventType::WalletCreated)
                .with_user("user_other")
                .with_resource("wallet", "w2"),
        )
        .unwrap();

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let events = repo.search_by_user("user_target", &today).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id, Some("user_target".to_string()));
    }

    #[test]
    fn search_by_resource() {
        let (_temp, storage) = setup();
        let repo = AuditRepository::new(&storage);

        repo.log(
            &AuditEvent::new(AuditEventType::WalletCreated)
                .with_user("user_1")
                .with_resource("wallet", "target_wallet"),
        )
        .unwrap();

        repo.log(
            &AuditEvent::new(AuditEventType::WalletAccessed)
                .with_user("user_2")
                .with_resource("wallet", "target_wallet"),
        )
        .unwrap();

        repo.log(
            &AuditEvent::new(AuditEventType::BookmarkCreated)
                .with_user("user_1")
                .with_resource("bookmark", "b1"),
        )
        .unwrap();

        let today = Utc::now().format("%Y-%m-%d").to_string();
        let events = repo
            .search_by_resource("wallet", "target_wallet", &today)
            .unwrap();

        assert_eq!(events.len(), 2);
    }
}
