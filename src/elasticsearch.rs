use std::{collections::HashMap, error::Error};

use hyper::{body::Bytes, http::header::CONTENT_TYPE, Body, Request};
use tracing::info;

use crate::common::make_component_request;

#[derive(serde::Serialize, Debug)]
pub struct SnapshotRequest {
    pub indices: String,
    pub feature_states: Vec<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SnapshotRepository {
    pub r#type: String,
}

async fn make_elasticsearch_request(
    kube: &kube::Client,
    req: Request<Body>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app=elasticsearch-master", 9200, req).await
}

#[tracing::instrument(skip(kube), err)]
pub async fn take_snapshot(
    kube: &kube::Client,
    req: SnapshotRequest,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    let repo = find_snapshot_repository(kube).await?;
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/_snapshot/{}/{}?wait_for_completion=true",
            repo, name
        ))
        .header(CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_string(&req)
                .expect("Snapshot request must be serializable")
                .into(),
        )?;

    make_elasticsearch_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn restore_snapshot(kube: &kube::Client, name: &str) -> Result<(), Box<dyn Error>> {
    let repo = find_snapshot_repository(kube).await?;
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "/_snapshot/{}/{}/_restore?wait_for_completion=true",
            repo, name
        ))
        .body(Body::empty())?;

    make_elasticsearch_request(kube, req).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
pub async fn get_all_indices(kube: &kube::Client) -> Result<Vec<String>, Box<dyn Error>> {
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Index {}

    let req = Request::builder()
        .uri("/*")
        .method("GET")
        .body(Body::empty())
        .expect("Request must be valid");
    let indices: std::collections::HashMap<String, Index> =
        serde_json::from_slice(&make_elasticsearch_request(kube, req).await?)?;
    Ok(indices.keys().cloned().collect())
}

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn delete_index(kube: &kube::Client, name: &str) -> Result<(), Box<dyn Error>> {
    let req = Request::builder()
        .uri(format!("/{name}"))
        .method("DELETE")
        .body(Body::empty())
        .expect("Request must be valid");

    make_elasticsearch_request(kube, req).await?;
    info!("Deleted index {}", name);
    Ok(())
}

#[tracing::instrument(skip(kube), err, level = "debug")]
async fn find_snapshot_repository(kube: &kube::Client) -> Result<String, Box<dyn Error>> {
    let req = Request::builder()
        .method("GET")
        .uri("/_snapshot/_all")
        .body(hyper::Body::empty())?;

    let resp = make_elasticsearch_request(kube, req).await?;

    let repoitories = serde_json::from_slice::<HashMap<String, SnapshotRepository>>(&resp)?;
    for (name, settings) in repoitories {
        tracing::debug!(
            "Found snapshot repository {} with settings {:?}",
            name,
            settings
        );
        if settings.r#type == "gcs" || settings.r#type == "s3" {
            tracing::debug!("Using repository {}", name);
            return Ok(name);
        }
    }
    Err("No snapshot repository found".into())
}
