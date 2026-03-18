use std::{
    collections::{BTreeSet, HashMap},
    error::Error,
};

use chrono::Utc;
use chrono::{LocalResult, TimeZone};

use chrono_humanize::HumanTime;
use tracing::{info, warn};

use crate::{
    operate,
    types::{
        BackupDescriptor, BackupState, OperateDetails, RuntimeBackupInfo, StorageMode, ZeebeDetails,
    },
    zeebe,
};

/// Trait for types that carry a backup ID and state, used to unify stats printing.
trait BackupEntry {
    fn backup_id(&self) -> u64;
    fn state(&self) -> BackupState;
}

impl<T> BackupEntry for BackupDescriptor<T> {
    fn backup_id(&self) -> u64 {
        self.backup_id
    }
    fn state(&self) -> BackupState {
        self.state
    }
}

impl BackupEntry for RuntimeBackupInfo {
    fn backup_id(&self) -> u64 {
        self.backup_id
    }
    fn state(&self) -> BackupState {
        self.state
    }
}

pub async fn list(storage_mode: StorageMode) -> Result<(), Box<dyn Error>> {
    let kube = kube::Client::try_default().await?;
    match storage_mode {
        StorageMode::Elasticsearch => list_es(&kube).await,
        StorageMode::Rdbms => list_rdbms(&kube).await,
    }
}

#[tracing::instrument(skip(kube), err)]
async fn list_es(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let zeebe_backups: Vec<BackupDescriptor<ZeebeDetails>> = zeebe::list_backups(kube).await?;
    let operate_backups = operate::list_backups(kube).await?;

    tracing::info_span!("Zeebe").in_scope(|| {
        print_stats("backups", &zeebe_backups);
    });
    tracing::info_span!("Operate").in_scope(|| {
        print_stats("backups", &operate_backups);
    });

    match find_most_recent_usable(&zeebe_backups, &operate_backups) {
        Some(id) => log_backup_timestamp("The most recent usable backup", id),
        None => warn!("No usable backups found"),
    }

    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn list_rdbms(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let runtime_backups = zeebe::list_runtime_backups(kube).await?;

    tracing::info_span!("Runtime Backups").in_scope(|| {
        print_stats("runtime backups", &runtime_backups);
    });

    match find_most_recent_runtime_backup(&runtime_backups) {
        Some(id) => log_backup_timestamp("The most recent completed runtime backup", id),
        None => warn!("No completed runtime backups found"),
    }

    // Show checkpoint state
    match zeebe::get_backup_state(kube).await {
        Ok(state) => {
            info!("Backup ranges: {} partition(s)", state.ranges.len());
        }
        Err(e) => {
            warn!("Could not fetch backup state: {}", e);
        }
    }

    Ok(())
}

fn log_backup_timestamp(label: &str, backup_id: u64) {
    info!("{} is {}", label, backup_id);
    if let LocalResult::Single(date) = Utc.timestamp_opt(backup_id as i64, 0) {
        info!(
            "This backup was created {} at {}",
            HumanTime::from(date),
            date
        );
    }
}

#[tracing::instrument(level = "debug")]
pub fn find_most_recent_usable(
    zeebe: &Vec<BackupDescriptor<ZeebeDetails>>,
    operate: &Vec<BackupDescriptor<OperateDetails>>,
) -> Option<u64> {
    let zeebe: BTreeSet<u64> = zeebe
        .iter()
        .filter(|b| b.state == BackupState::Completed)
        .map(|d| d.backup_id)
        .collect();
    let operate: BTreeSet<u64> = operate
        .iter()
        .filter(|b| b.state == BackupState::Completed)
        .map(|d| d.backup_id)
        .collect();

    zeebe.intersection(&operate).last().copied()
}

pub fn find_most_recent_runtime_backup(backups: &[RuntimeBackupInfo]) -> Option<u64> {
    backups
        .iter()
        .filter(|b| b.state == BackupState::Completed)
        .map(|b| b.backup_id)
        .max()
}

fn print_stats<T: BackupEntry>(label: &str, backups: &[T]) {
    let backups_by_state =
        backups
            .iter()
            .fold(HashMap::<BackupState, Vec<&T>>::new(), |mut map, backup| {
                map.entry(backup.state()).or_default().push(backup);
                map
            });

    for (state, mut entries) in backups_by_state {
        entries.sort_by_key(|e| e.backup_id());
        let most_recent = entries
            .iter()
            .rev()
            .take(3)
            .map(|e| e.backup_id().to_string())
            .collect::<Vec<_>>();
        info!(
            "{} {} {:?}: {}, ...",
            entries.len(),
            label,
            state,
            most_recent.join(", ")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        BackupDescriptor, BackupState, OperateDetails, RuntimeBackupInfo, ZeebeDetails,
    };

    #[test]
    fn test_find_most_recent_usable_empty() {
        let zeebe: Vec<BackupDescriptor<ZeebeDetails>> = vec![];
        let operate: Vec<BackupDescriptor<OperateDetails>> = vec![];
        assert_eq!(find_most_recent_usable(&zeebe, &operate), None);
    }

    #[test]
    fn test_find_most_recent_usable_no_overlap() {
        let zeebe = vec![BackupDescriptor {
            backup_id: 1,
            state: BackupState::Completed,
            details: vec![],
        }];
        let operate = vec![BackupDescriptor {
            backup_id: 2,
            state: BackupState::Completed,
            details: vec![],
        }];
        assert_eq!(find_most_recent_usable(&zeebe, &operate), None);
    }

    #[test]
    fn test_find_most_recent_usable_with_overlap() {
        let zeebe = vec![
            BackupDescriptor {
                backup_id: 1,
                state: BackupState::Completed,
                details: vec![],
            },
            BackupDescriptor {
                backup_id: 2,
                state: BackupState::Completed,
                details: vec![],
            },
        ];
        let operate = vec![
            BackupDescriptor {
                backup_id: 2,
                state: BackupState::Completed,
                details: vec![],
            },
            BackupDescriptor {
                backup_id: 3,
                state: BackupState::Completed,
                details: vec![],
            },
        ];
        assert_eq!(find_most_recent_usable(&zeebe, &operate), Some(2));
    }

    #[test]
    fn test_find_most_recent_usable_ignores_non_completed() {
        let zeebe = vec![
            BackupDescriptor {
                backup_id: 1,
                state: BackupState::Completed,
                details: vec![],
            },
            BackupDescriptor {
                backup_id: 2,
                state: BackupState::Failed,
                details: vec![],
            },
        ];
        let operate = vec![
            BackupDescriptor {
                backup_id: 1,
                state: BackupState::Completed,
                details: vec![],
            },
            BackupDescriptor {
                backup_id: 2,
                state: BackupState::Completed,
                details: vec![],
            },
        ];
        assert_eq!(find_most_recent_usable(&zeebe, &operate), Some(1));
    }

    #[test]
    fn test_find_most_recent_runtime_backup_empty() {
        let backups: Vec<RuntimeBackupInfo> = vec![];
        assert_eq!(find_most_recent_runtime_backup(&backups), None);
    }

    #[test]
    fn test_find_most_recent_runtime_backup_finds_latest_completed() {
        let backups = vec![
            RuntimeBackupInfo {
                backup_id: 100,
                state: BackupState::Completed,
                failure_reason: None,
                details: vec![],
            },
            RuntimeBackupInfo {
                backup_id: 200,
                state: BackupState::Failed,
                failure_reason: None,
                details: vec![],
            },
            RuntimeBackupInfo {
                backup_id: 150,
                state: BackupState::Completed,
                failure_reason: None,
                details: vec![],
            },
        ];
        assert_eq!(find_most_recent_runtime_backup(&backups), Some(150));
    }
}
