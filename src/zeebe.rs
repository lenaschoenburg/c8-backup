use std::error::Error;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{header::CONTENT_TYPE, Request};

use crate::{
    common::make_component_request,
    types::{
        BackupDescriptor, CheckpointState, RuntimeBackupInfo, TakeBackupRequest,
        TakeRuntimeBackupRequest, ZeebeDetails,
    },
};

#[tracing::instrument(skip(kube), err)]
pub async fn take_backup(kube: &kube::Client, backup_id: u64) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/backups")
        .header(CONTENT_TYPE, "application/json")
        .body(Full::from(
            serde_json::to_string(&TakeBackupRequest {
                backup_id: backup_id.to_string(),
            })
            .expect("Request can be serialized"),
        ))?;
    make_zeebe_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn query_backup(
    kube: &kube::Client,
    backup_id: u64,
) -> Result<BackupDescriptor<ZeebeDetails>, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/actuator/backups/{}", backup_id))
        .body(Full::default())?;

    let resp = make_zeebe_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn list_backups(
    kube: &kube::Client,
) -> Result<Vec<BackupDescriptor<ZeebeDetails>>, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri("/actuator/backups")
        .body(Full::default())?;
    let resp = make_zeebe_request(kube, req).await?;

    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err)]
pub async fn pause_exporting(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/exporting/pause")
        .body(Full::default())?;

    make_zeebe_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
pub async fn resume_exporting(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/exporting/resume")
        .body(Full::default())?;

    make_zeebe_request(kube, req).await?;
    Ok(())
}

async fn make_zeebe_request(
    kube: &kube::Client,
    req: Request<Full<Bytes>>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app.kubernetes.io/component=zeebe-gateway", 9600, req).await
}

// --- RDBMS Runtime Backup API ---

#[tracing::instrument(skip(kube), err)]
pub async fn take_runtime_backup(
    kube: &kube::Client,
    backup_id: u64,
) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/backupRuntime")
        .header(CONTENT_TYPE, "application/json")
        .body(Full::from(
            serde_json::to_string(&TakeRuntimeBackupRequest { backup_id })
                .expect("Request can be serialized"),
        ))?;
    make_zeebe_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn query_runtime_backup(
    kube: &kube::Client,
    backup_id: u64,
) -> Result<RuntimeBackupInfo, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri(format!("/actuator/backupRuntime/{}", backup_id))
        .body(Full::default())?;
    let resp = make_zeebe_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn list_runtime_backups(
    kube: &kube::Client,
) -> Result<Vec<RuntimeBackupInfo>, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri("/actuator/backupRuntime")
        .body(Full::default())?;
    let resp = make_zeebe_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn get_backup_state(kube: &kube::Client) -> Result<CheckpointState, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri("/actuator/backupRuntime/state")
        .body(Full::default())?;
    let resp = make_zeebe_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}
