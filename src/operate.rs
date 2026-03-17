use std::error::Error;

use hyper::{body::Bytes, http::header::CONTENT_TYPE, Body, Request};

use crate::{
    common::make_component_request,
    types::{BackupDescriptor, HistoryBackupInfo, OperateDetails, TakeBackupRequest},
};

async fn make_operate_request(
    kube: &kube::Client,
    req: Request<Body>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app.kubernetes.io/component=operate", 8080, req).await
}

#[allow(dead_code)]
async fn make_management_request(
    kube: &kube::Client,
    req: Request<Body>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app.kubernetes.io/component=zeebe-gateway", 9600, req).await
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub(crate) async fn list_backups(
    kube: &kube::Client,
) -> Result<Vec<BackupDescriptor<OperateDetails>>, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/actuator/backups"))
        .body(Body::empty())?;

    let resp = make_operate_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn query_backup(
    kube: &kube::Client,
    backup_id: u64,
) -> Result<BackupDescriptor<OperateDetails>, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/actuator/backups/{}", backup_id))
        .body(Body::empty())?;

    let resp = make_operate_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err)]
pub async fn take_backup(kube: &kube::Client, backup_id: u64) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/backups")
        .header(CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(&TakeBackupRequest {
                backup_id: backup_id.to_string(),
            })
            .expect("Backup must be serializable")
            .into(),
        )?;

    make_operate_request(kube, req).await?;
    Ok(())
}

// --- RDBMS History Backup API ---

#[tracing::instrument(skip(kube), err)]
pub async fn take_history_backup(
    kube: &kube::Client,
    backup_id: u64,
) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/backupHistory")
        .header(CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(&TakeBackupRequest {
                backup_id: backup_id.to_string(),
            })
            .expect("Backup must be serializable")
            .into(),
        )?;
    make_management_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn query_history_backup(
    kube: &kube::Client,
    backup_id: u64,
) -> Result<HistoryBackupInfo, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/actuator/backupHistory/{}", backup_id))
        .body(Body::empty())?;
    let resp = make_management_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn list_history_backups(
    kube: &kube::Client,
) -> Result<Vec<HistoryBackupInfo>, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri("/actuator/backupHistory")
        .body(Body::empty())?;
    let resp = make_management_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}
