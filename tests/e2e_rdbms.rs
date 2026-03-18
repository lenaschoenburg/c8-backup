//! E2e tests for RDBMS storage mode.
//!
//! Requires a running Kubernetes cluster with Camunda 8 deployed in RDBMS mode
//! (PostgreSQL + continuous backups enabled).
//!
//! Run with: cargo test --test e2e_rdbms -- --nocapture --ignored

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
async fn rdbms_list_runtime_backups() {
    require_cluster!();
    let result = c8_backup::list::list(StorageMode::Rdbms).await;
    assert!(result.is_ok(), "list failed: {:?}", result.err());
}

#[tokio::test]
#[ignore]
async fn rdbms_create_runtime_backup() {
    require_cluster!();

    let result = c8_backup::create::create(StorageMode::Rdbms).await;
    assert!(result.is_ok(), "create failed: {:?}", result.err());

    let result = c8_backup::list::list(StorageMode::Rdbms).await;
    assert!(
        result.is_ok(),
        "list after create failed: {:?}",
        result.err()
    );
}
