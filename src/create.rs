use std::{error::Error, time::Duration};

use chrono::Utc;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
    elasticsearch::{take_snapshot, SnapshotRequest},
    operate,
    types::{BackupDescriptor, BackupState},
    zeebe,
};

#[tracing::instrument(err)]
pub(crate) async fn create() -> Result<(), Box<dyn Error>> {
    let kube = kube::Client::try_default().await?;
    let backup_id = Utc::now().timestamp() as u64;

    let result = try_backup(&kube, backup_id).await;
    match result {
        Err(e) => {
            warn!(e, "Backup failed, trying to resume Zeebe exporting");
            zeebe::resume_exporting(&kube).await?;
            Err(e)
        }
        _ => result,
    }
}

#[tracing::instrument(skip(kube), err)]
pub(crate) async fn try_backup(kube: &kube::Client, backup_id: u64) -> Result<(), Box<dyn Error>> {
    backup_operate(&kube, backup_id).await?;
    zeebe::pause_exporting(&kube).await?;
    backup_zeebe_export(&kube, backup_id).await?;
    backup_zeebe(&kube, backup_id).await?;
    zeebe::resume_exporting(&kube).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn backup_operate(kube: &kube::Client, backup_id: u64) -> Result<(), Box<dyn Error>> {
    operate::take_backup(&kube, backup_id).await?;

    info!("Started backup");
    loop {
        match operate::query_backup(kube, backup_id).await {
            Ok(BackupDescriptor {
                state: BackupState::Completed,
                ..
            }) => {
                info!("Backup completed");
                return Ok(());
            }
            result => {
                info!(
                    "Checking again in 5 seconds, state is {}",
                    result
                        .as_ref()
                        .map(|b| format!("{:?}", b.state))
                        .unwrap_or(format!("{:?}", result))
                );
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        }
    }
}

#[tracing::instrument(skip(kube), err)]
async fn backup_zeebe_export(kube: &kube::Client, backup_id: u64) -> Result<(), Box<dyn Error>> {
    let req = SnapshotRequest {
        indices: "zeebe-record*".into(),
        feature_states: vec!["none".into()],
    };
    let name = format!("camunda_zeebe_records_{}", backup_id);
    take_snapshot(kube, req, &name).await?;

    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn backup_zeebe(kube: &kube::Client, backup_id: u64) -> Result<(), Box<dyn Error>> {
    zeebe::take_backup(kube, backup_id).await?;
    info!("Started backup");
    loop {
        match zeebe::query_backup(kube, backup_id).await {
            Ok(BackupDescriptor {
                state: BackupState::Completed,
                ..
            }) => {
                info!("Backup completed");
                return Ok(());
            }
            result => {
                info!(
                    "Checking again in 5 seconds, state is {}",
                    result
                        .as_ref()
                        .map(|b| format!("{:?}", b.state))
                        .unwrap_or(format!("{:?}", result))
                );
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        }
    }
}
