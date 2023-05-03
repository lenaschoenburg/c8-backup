use std::{
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};

use hyper::{
    http::{self, Request},
    Body,
};
use tracing::{info, warn};

use crate::{
    common::{make_elasticsearch_request, make_operate_request, make_zeebe_request},
    types::{Backup, BackupDescriptor, BackupState, OperateDetails, ZeebeDetails},
};

#[tracing::instrument(err)]
pub(crate) async fn backup() -> Result<(), Box<dyn Error>> {
    let kube = kube::Client::try_default().await?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let new_backup = Backup {
        backup_id: timestamp,
    };

    let result = try_backup(&kube, &new_backup).await;
    match result {
        Err(e) => {
            warn!(e, "Backup failed, trying to resume Zeebe exporting");
            resume_exporting(&kube).await?;
            Err(e)
        }
        _ => result,
    }
}

#[tracing::instrument(skip(kube), err)]
pub(crate) async fn try_backup(
    kube: &kube::Client,
    new_backup: &Backup,
) -> Result<(), Box<dyn Error>> {
    backup_operate(&kube, &new_backup).await?;
    pause_exporting(&kube).await?;
    backup_zeebe_export(&kube, &new_backup).await?;
    backup_zeebe(&kube, &new_backup).await?;
    resume_exporting(&kube).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn backup_operate(kube: &kube::Client, new_backup: &Backup) -> Result<(), Box<dyn Error>> {
    let take_backup = Request::builder()
        .method("POST")
        .uri("/actuator/backups")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(new_backup)
                .expect("Backup must be serializable")
                .into(),
        )?;

    make_operate_request(kube, take_backup).await?;
    info!("Started backup");
    let mut backup: BackupDescriptor<OperateDetails>;
    loop {
        let query_backup = Request::builder()
            .uri(format!("/actuator/backups/{}", new_backup.backup_id))
            .body(Body::empty())?;

        match make_operate_request(kube, query_backup).await {
            Ok(response) => {
                backup = serde_json::from_slice(&response)?;
                match backup.state {
                    BackupState::Completed => {
                        info!("Backup completed");
                        break;
                    }
                    BackupState::Failed => {
                        return Err("Backup failed".into());
                    }
                    _ => {
                        info!(?backup.state, "Checking again in 5 seconds");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
            Err(e) => {
                warn!(?e, "Backup status unknown, trying again in 5 seconds");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn pause_exporting(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/exporting/pause")
        .body(Body::empty())?;

    make_zeebe_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn resume_exporting(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/exporting/resume")
        .body(Body::empty())?;

    make_zeebe_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn backup_zeebe_export(
    kube: &kube::Client,
    new_backup: &Backup,
) -> Result<(), Box<dyn Error>> {
    #[derive(serde::Serialize)]
    struct SnapshotRequest {
        indices: String,
        feature_states: Vec<String>,
    }
    let req = SnapshotRequest {
        indices: "zeebe-record*".into(),
        feature_states: vec!["none".into()],
    };
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/_snapshot/gcs/camunda_zeebe_records_{}?wait_for_completion=true",
            new_backup.backup_id
        ))
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(&req)
                .expect("Snapshot request must be serializable")
                .into(),
        )?;

    make_elasticsearch_request(kube, req).await?;

    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn backup_zeebe(kube: &kube::Client, new_backup: &Backup) -> Result<(), Box<dyn Error>> {
    let take_backup = Request::builder()
        .method("POST")
        .uri("/actuator/backups")
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(new_backup)
                .expect("Backup must be serializable")
                .into(),
        )?;

    make_zeebe_request(kube, take_backup).await?;
    info!("Started backup");
    let mut backup: BackupDescriptor<ZeebeDetails>;
    loop {
        let query_backup = Request::builder()
            .uri(format!("/actuator/backups/{}", new_backup.backup_id))
            .body(Body::empty())?;

        backup = serde_json::from_slice(&make_zeebe_request(kube, query_backup).await?)?;
        match backup.state {
            BackupState::Completed => {
                info!("Backup completed");
                break;
            }
            BackupState::Failed => {
                return Err("Backup failed".into());
            }
            _ => {
                info!(?backup.state, "Checking again in 5 seconds");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}
