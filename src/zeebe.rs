use std::error::Error;

use hyper::{body::Bytes, header::CONTENT_TYPE, Body, Request};

use crate::{
    common::make_component_request,
    types::{BackupDescriptor, TakeBackupRequest, ZeebeDetails},
};

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
            .expect("Request can be serialized")
            .into(),
        )?;
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
        .body(Body::empty())?;

    let resp = make_zeebe_request(kube, req).await?;
    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err)]
pub async fn list_backups(
    kube: &kube::Client,
) -> Result<Vec<BackupDescriptor<ZeebeDetails>>, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri("/actuator/backups")
        .body(Body::empty())?;
    let resp = make_zeebe_request(kube, req).await?;

    Ok(serde_json::from_slice(&resp)?)
}

#[tracing::instrument(skip(kube), err)]
pub async fn pause_exporting(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/exporting/pause")
        .body(Body::empty())?;

    make_zeebe_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
pub async fn resume_exporting(kube: &kube::Client) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .method("POST")
        .uri("/actuator/exporting/resume")
        .body(Body::empty())?;

    make_zeebe_request(kube, req).await?;
    Ok(())
}

async fn make_zeebe_request(
    kube: &kube::Client,
    req: Request<Body>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app.kubernetes.io/component=zeebe-gateway", 9600, req).await
}
