//! E2e tests for Elasticsearch storage mode.
//!
//! Requires a running Kubernetes cluster with Camunda 8 deployed in ES mode.
//! Run `./scripts/e2e-setup.sh` first, or set up your own cluster.
//!
//! Run with: cargo test --test e2e_es -- --nocapture --ignored

mod e2e;

use c8_backup::types::StorageMode;

macro_rules! require_cluster {
    () => {
        if e2e::try_kube_client().await.is_none() {
            eprintln!("SKIPPED: no Kubernetes cluster available");
            return;
        }
    };
}

#[tokio::test]
#[ignore]
async fn es_list_backups() {
    require_cluster!();
    let result = c8_backup::list::list(StorageMode::Elasticsearch).await;
    assert!(result.is_ok(), "list failed: {:?}", result.err());
}

#[tokio::test]
#[ignore]
async fn es_create_and_list_backup() {
    require_cluster!();

    let result = c8_backup::create::create(StorageMode::Elasticsearch).await;
    assert!(result.is_ok(), "create failed: {:?}", result.err());

    let result = c8_backup::list::list(StorageMode::Elasticsearch).await;
    assert!(
        result.is_ok(),
        "list after create failed: {:?}",
        result.err()
    );
}
