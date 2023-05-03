use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupState {
    Completed,
    Failed,
    InProgress,
    Incomplete,
    DoesNotExist,
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
pub struct Backup {
    pub backup_id: String,
}
