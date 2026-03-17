use serde::{Deserialize, Serialize};

// --- StorageMode enum for CLI ---

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum StorageMode {
    Elasticsearch,
    Rdbms,
}

// --- Existing types (unchanged) ---

#[derive(Deserialize, Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupState {
    Completed,
    Failed,
    InProgress,
    Incomplete,
    DoesNotExist,
    Incompatible,
    Deleted,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupDescriptor<T> {
    pub backup_id: u64,
    pub state: BackupState,
    pub details: Vec<T>,
}

#[derive(Deserialize, Debug)]
pub struct ZeebeDetails {}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OperateDetails {
    pub snapshot_name: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TakeBackupRequest {
    pub backup_id: String,
}

// --- Runtime backup API types (GET /actuator/backupRuntime) ---

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBackupInfo {
    pub backup_id: u64,
    pub state: BackupState,
    #[serde(default)]
    pub failure_reason: Option<String>,
    pub details: Vec<PartitionBackupInfo>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PartitionBackupInfo {
    pub partition_id: u32,
    pub state: BackupState,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub last_updated_at: Option<String>,
    #[serde(default)]
    pub snapshot_id: Option<String>,
    #[serde(default)]
    pub checkpoint_position: Option<i64>,
    #[serde(default)]
    pub broker_id: Option<i32>,
    #[serde(default)]
    pub broker_version: Option<String>,
}

// --- History backup API types (GET /actuator/backupHistory) ---

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryBackupInfo {
    pub backup_id: u64,
    pub state: BackupState,
    #[serde(default)]
    pub failure_reason: Option<String>,
    pub details: Vec<HistoryBackupDetail>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryBackupDetail {
    pub snapshot_name: String,
    pub state: String,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub failures: Vec<String>,
}

// --- Checkpoint state (GET /actuator/backupRuntime/state) ---

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointState {
    #[serde(default)]
    pub checkpoint_states: Vec<serde_json::Value>,
    #[serde(default)]
    pub backup_states: Vec<serde_json::Value>,
    #[serde(default)]
    pub ranges: Vec<serde_json::Value>,
}

// --- Internal restore target enum ---

#[derive(Debug)]
pub enum RestoreTarget {
    EsBackup { id: u64, snapshots: Vec<String> },
    RdbmsAuto,
    RdbmsBackupId { id: u64 },
    RdbmsPointInTime { to: String },
}

// --- Request type for runtime backups ---

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TakeRuntimeBackupRequest {
    pub backup_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_state_deserialize() {
        let cases = vec![
            (r#""COMPLETED""#, BackupState::Completed),
            (r#""IN_PROGRESS""#, BackupState::InProgress),
            (r#""FAILED""#, BackupState::Failed),
            (r#""INCOMPLETE""#, BackupState::Incomplete),
            (r#""DOES_NOT_EXIST""#, BackupState::DoesNotExist),
            (r#""INCOMPATIBLE""#, BackupState::Incompatible),
            (r#""DELETED""#, BackupState::Deleted),
        ];
        for (json, expected) in cases {
            let state: BackupState = serde_json::from_str(json).unwrap();
            assert_eq!(state, expected);
        }
    }

    #[test]
    fn test_runtime_backup_info_deserialize() {
        let json = r#"{
            "backupId": 1683214620,
            "state": "COMPLETED",
            "details": [{
                "partitionId": 1,
                "state": "COMPLETED",
                "createdAt": "2022-09-15T13:10:38.176514094Z",
                "snapshotId": "238632143-55-690906332-690905294",
                "checkpointPosition": 10,
                "brokerId": 0,
                "brokerVersion": "8.1.2"
            }]
        }"#;
        let info: RuntimeBackupInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.backup_id, 1683214620);
        assert_eq!(info.state, BackupState::Completed);
        assert_eq!(info.details.len(), 1);
        assert_eq!(info.details[0].partition_id, 1);
    }

    #[test]
    fn test_runtime_backup_info_with_failure() {
        let json = r#"{
            "backupId": 100,
            "state": "FAILED",
            "failureReason": "disk full",
            "details": []
        }"#;
        let info: RuntimeBackupInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.state, BackupState::Failed);
        assert_eq!(info.failure_reason.as_deref(), Some("disk full"));
    }

    #[test]
    fn test_history_backup_info_deserialize() {
        let json = r#"{
            "backupId": 1683214620,
            "state": "COMPLETED",
            "details": [{
                "snapshotName": "camunda_operate_1683214620_8.2.0_part_1_of_6",
                "state": "SUCCESS",
                "startTime": "2023-01-01T10:10:10.100+0000",
                "failures": []
            }]
        }"#;
        let info: HistoryBackupInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.backup_id, 1683214620);
        assert_eq!(
            info.details[0].snapshot_name,
            "camunda_operate_1683214620_8.2.0_part_1_of_6"
        );
    }

    #[test]
    fn test_checkpoint_state_deserialize() {
        let json = r#"{"checkpointStates": [], "backupStates": [], "ranges": []}"#;
        let state: CheckpointState = serde_json::from_str(json).unwrap();
        assert!(state.checkpoint_states.is_empty());
    }

    #[test]
    fn test_checkpoint_state_default() {
        let json = r#"{}"#;
        let state: CheckpointState = serde_json::from_str(json).unwrap();
        assert!(state.checkpoint_states.is_empty());
    }

    #[test]
    fn test_existing_zeebe_backup_descriptor_still_works() {
        let json = r#"{"backupId": 123, "state": "COMPLETED", "details": [{}]}"#;
        let desc: BackupDescriptor<ZeebeDetails> = serde_json::from_str(json).unwrap();
        assert_eq!(desc.backup_id, 123);
    }

    #[test]
    fn test_existing_operate_backup_descriptor_still_works() {
        let json =
            r#"{"backupId": 456, "state": "IN_PROGRESS", "details": [{"snapshotName": "snap1"}]}"#;
        let desc: BackupDescriptor<OperateDetails> = serde_json::from_str(json).unwrap();
        assert_eq!(desc.backup_id, 456);
    }

    #[test]
    fn test_take_runtime_backup_request_serialize() {
        let req = TakeRuntimeBackupRequest { backup_id: 42 };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, r#"{"backupId":42}"#);
    }
}
