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

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBackupInfo {
    pub backup_id: u64,
    pub state: BackupState,
    #[serde(default)]
    pub failure_reason: Option<String>,
    pub details: Vec<PartitionBackupInfo>,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryBackupInfo {
    pub backup_id: u64,
    pub state: BackupState,
    #[serde(default)]
    pub failure_reason: Option<String>,
    pub details: Vec<HistoryBackupDetail>,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug)]
pub enum RestoreTarget {
    EsBackup { id: u64, snapshots: Vec<String> },
    RdbmsAuto,
    RdbmsBackupId { id: u64 },
    RdbmsPointInTime { to: String },
}

// --- Request type for runtime backups ---

#[allow(dead_code)]
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TakeRuntimeBackupRequest {
    pub backup_id: u64,
}
