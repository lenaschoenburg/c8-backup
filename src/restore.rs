use std::collections::HashMap;

use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::{Job, JobSpec},
    core::v1::{
        Container, EnvVar, PersistentVolumeClaim, PersistentVolumeClaimVolumeSource, PodSpec,
        PodTemplateSpec, Volume, VolumeMount,
    },
};
use kube::{
    api::{DeleteParams, ListParams, Patch, PatchParams, PostParams},
    core::ObjectMeta,
    runtime::{conditions, wait::await_condition},
    Api,
};

use serde_json::json;
use tracing::info;

use crate::{
    elasticsearch::{delete_index, get_all_indices, restore_snapshot},
    list, operate,
    types::{RestoreTarget, StorageMode},
    zeebe,
};

#[derive(Debug)]
struct Backup {
    id: u64,
    snapshots: Vec<String>,
}

#[tracing::instrument(err)]
pub(crate) async fn restore(
    storage_mode: StorageMode,
    to: Option<String>,
    backup_id: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let kube = kube::Client::try_default().await?;

    match storage_mode {
        StorageMode::Elasticsearch => restore_es(&kube).await,
        StorageMode::Rdbms => {
            let target = determine_restore_target(to, backup_id)?;
            restore_rdbms(&kube, &target).await
        }
    }
}

fn determine_restore_target(
    to: Option<String>,
    backup_id: Option<u64>,
) -> Result<RestoreTarget, Box<dyn std::error::Error>> {
    match (to, backup_id) {
        (Some(_), Some(_)) => Err("Cannot specify both --to and --backup-id".into()),
        (Some(ts), None) => Ok(RestoreTarget::RdbmsPointInTime { to: ts }),
        (None, Some(id)) => Ok(RestoreTarget::RdbmsBackupId { id }),
        (None, None) => Ok(RestoreTarget::RdbmsAuto),
    }
}

fn restore_args_for_target(target: &RestoreTarget) -> Vec<String> {
    match target {
        RestoreTarget::RdbmsAuto => vec![],
        RestoreTarget::RdbmsBackupId { id } => vec![format!("--backupId={}", id)],
        RestoreTarget::RdbmsPointInTime { to } => vec![format!("--to={}", to)],
        RestoreTarget::EsBackup { id, .. } => vec![format!("--backupId={}", id)],
    }
}

#[tracing::instrument(skip(kube), err)]
async fn restore_es(kube: &kube::Client) -> Result<(), Box<dyn std::error::Error>> {
    let backup = find_newest_backup(kube).await?;
    let restartable = shutdown_apps(kube).await?;

    delete_indices(kube).await?;
    restore_indices(kube, &backup).await?;

    delete_zeebe_data(kube).await?;
    restore_zeebe_data_es(kube, &backup).await?;

    start_apps(kube, &restartable).await?;
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn restore_rdbms(
    kube: &kube::Client,
    target: &RestoreTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    let restartable = shutdown_apps(kube).await?;

    // No ES index operations in RDBMS mode

    delete_zeebe_data(kube).await?;
    restore_zeebe_data_rdbms(kube, target).await?;

    start_apps(kube, &restartable).await?;
    Ok(())
}

fn zeebe_data_deletion_job(pvc: &PersistentVolumeClaim) -> Job {
    let name = pvc.metadata.name.to_owned().expect("PVC must have a name");
    Job {
        metadata: ObjectMeta {
            name: Some(format!("delete-{}", &name)),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: "delete-zeebe".to_string(),
                        image: Some("busybox:latest".to_string()),
                        command: Some(vec![
                            "/bin/sh".to_string(),
                            "-c".to_string(),
                            "rm -rf /usr/local/zeebe/data/*".to_string(),
                        ]),
                        volume_mounts: Some(vec![VolumeMount {
                            name: "data".to_string(),
                            mount_path: "/usr/local/zeebe/data".to_string(),
                            ..Default::default()
                        }]),
                        ..Default::default()
                    }],
                    volumes: Some(vec![Volume {
                        name: "data".to_string(),
                        persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                            claim_name: name,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }]),
                    restart_policy: Some("Never".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
        status: None,
    }
}

fn zeebe_data_restoration_job(
    restore_binary: &str,
    restore_args: &[String],
    pvc: &PersistentVolumeClaim,
    sfs: &StatefulSet,
) -> Job {
    let name = pvc.metadata.name.to_owned().expect("PVC must have a name");
    let (_, node) = name.rsplit_once('-').expect("PVC must end with '-NODEID'");
    let pod_spec = sfs
        .spec
        .as_ref()
        .expect("Zeebe StatefulSet must have a spec")
        .template
        .spec
        .as_ref()
        .expect("Zeebe PodSpec must have a spec");
    let container = pod_spec
        .containers
        .first()
        .expect("Zeebe PodSpec must have a container");

    let mut envs = container.env.clone().unwrap_or_default();
    envs.push(EnvVar {
        name: "ZEEBE_BROKER_CLUSTER_NODEID".to_string(),
        value: Some(node.to_string()),
        value_from: None,
    });

    let mut command = vec![restore_binary.to_string()];
    command.extend(restore_args.iter().cloned());

    Job {
        metadata: ObjectMeta {
            name: Some(format!("restore-{}", name)),
            ..Default::default()
        },
        spec: Some(JobSpec {
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    service_account_name: pod_spec.service_account_name.clone(),
                    containers: vec![Container {
                        name: "restore-zeebe".to_string(),
                        image: Some(
                            container
                                .image
                                .clone()
                                .expect("Zeebe container must define an image"),
                        ),
                        command: Some(command),
                        volume_mounts: Some(vec![VolumeMount {
                            name: "data".to_string(),
                            mount_path: "/usr/local/zeebe/data".to_string(),
                            ..Default::default()
                        }]),
                        env: Some(envs),
                        ..Default::default()
                    }],
                    volumes: Some(vec![Volume {
                        name: "data".to_string(),
                        persistent_volume_claim: Some(PersistentVolumeClaimVolumeSource {
                            claim_name: name,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }]),
                    restart_policy: Some("Never".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
        status: None,
    }
}

#[tracing::instrument(skip(kube), err)]
async fn delete_zeebe_data(kube: &kube::Client) -> Result<(), Box<dyn std::error::Error>> {
    let jobs: Api<Job> = Api::default_namespaced(kube.clone());
    let pvcs: Api<PersistentVolumeClaim> = Api::default_namespaced(kube.clone());
    let zeebe_pvcs = pvcs
        .list(&ListParams::default().labels("app.kubernetes.io/component=zeebe-broker"))
        .await?;

    for pvc in &zeebe_pvcs {
        let pvc_name = pvc.metadata.name.to_owned().expect("PVC must have a name");
        let job = zeebe_data_deletion_job(pvc);
        jobs.create(&PostParams::default(), &job).await?;
        info!("Deleting data of {}", pvc_name)
    }

    for pvc in &zeebe_pvcs {
        let pvc_name = pvc.metadata.name.to_owned().expect("PVC must have a name");
        let job_name = format!("delete-{}", pvc_name);
        await_condition(jobs.clone(), &job_name, conditions::is_job_completed()).await?;
        jobs.delete(&job_name, &DeleteParams::background()).await?;
        info!("Deleted data of {}", pvc_name);
    }
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn restore_zeebe_data_es(
    kube: &kube::Client,
    backup: &Backup,
) -> Result<(), Box<dyn std::error::Error>> {
    let restore_args = vec![format!("--backupId={}", backup.id)];
    restore_zeebe_data_with_args(kube, "/usr/local/zeebe/bin/restore", &restore_args).await
}

#[tracing::instrument(skip(kube), err)]
async fn restore_zeebe_data_rdbms(
    kube: &kube::Client,
    target: &RestoreTarget,
) -> Result<(), Box<dyn std::error::Error>> {
    let restore_args = restore_args_for_target(target);
    restore_zeebe_data_with_args(kube, "/usr/local/camunda/bin/restore", &restore_args).await
}

async fn restore_zeebe_data_with_args(
    kube: &kube::Client,
    restore_binary: &str,
    restore_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let jobs: Api<Job> = Api::default_namespaced(kube.clone());
    let sfs: Api<StatefulSet> = Api::default_namespaced(kube.clone());

    let zeebe = {
        let sfs = sfs
            .list(&ListParams::default().labels("app.kubernetes.io/component=zeebe-broker"))
            .await?
            .items;
        sfs.first().expect("Zeebe StatefulSet must exist").clone()
    };

    let pvcs: Api<PersistentVolumeClaim> = Api::default_namespaced(kube.clone());
    let zeebe_pvcs = pvcs
        .list(&ListParams::default().labels("app.kubernetes.io/component=zeebe-broker"))
        .await?;

    for pvc in &zeebe_pvcs {
        let pvc_name = pvc.metadata.name.to_owned().expect("PVC must have a name");
        let job = zeebe_data_restoration_job(restore_binary, restore_args, pvc, &zeebe);
        jobs.create(&PostParams::default(), &job).await?;
        info!("Restoring data of {}", pvc_name)
    }
    for pvc in &zeebe_pvcs {
        let pvc_name = pvc.metadata.name.to_owned().expect("PVC must have a name");
        let job_name = format!("restore-{}", pvc_name);
        await_condition(jobs.clone(), &job_name, conditions::is_job_completed()).await?;
        jobs.delete(&job_name, &DeleteParams::background()).await?;
        info!("Restored data of {}", pvc_name);
    }
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn delete_indices(kube: &kube::Client) -> Result<(), Box<dyn std::error::Error>> {
    let indices = get_all_indices(kube).await?;

    for index in indices {
        delete_index(kube, &index).await?;
    }
    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn restore_indices(
    kube: &kube::Client,
    backup: &Backup,
) -> Result<(), Box<dyn std::error::Error>> {
    for snapshot in &backup.snapshots {
        restore_snapshot(kube, snapshot).await?;
        info!("Restored snapshot {}", snapshot);
    }
    Ok(())
}

struct RestartableApps {
    deployments: HashMap<String, i32>,
    statefulsets: HashMap<String, i32>,
}

#[tracing::instrument(skip(kube), err)]
async fn shutdown_apps(kube: &kube::Client) -> Result<RestartableApps, Box<dyn std::error::Error>> {
    let deploy: Api<Deployment> = Api::default_namespaced(kube.clone());
    let sfs: Api<StatefulSet> = Api::default_namespaced(kube.clone());

    let deployments: HashMap<String, i32> = deploy
        .list(&ListParams::default().labels("app.kubernetes.io/part-of=camunda-platform"))
        .await?
        .iter()
        .map(|deploy| {
            let replicas = deploy
                .spec
                .as_ref()
                .expect("Deployment must have a spec")
                .replicas
                .expect("Deployment must have replicas configured");
            (
                deploy
                    .metadata
                    .name
                    .to_owned()
                    .expect("Deploymetn must have a name"),
                replicas,
            )
        })
        .collect();
    let statefulsets: HashMap<String, i32> = sfs
        .list(&ListParams::default().labels("app.kubernetes.io/part-of=camunda-platform"))
        .await?
        .iter()
        .map(|statefulset| {
            let replicas = statefulset
                .spec
                .as_ref()
                .expect("StatefulSet must have a spec")
                .replicas
                .expect("StatefulSet must have replicas configured");
            (
                statefulset
                    .metadata
                    .name
                    .to_owned()
                    .expect("Deploymetn must have a name"),
                replicas,
            )
        })
        .collect();

    for name in deployments.keys() {
        deploy
            .patch_scale(
                name,
                &PatchParams::default(),
                &Patch::Merge(&json!({"spec": {"replicas": 0}})),
            )
            .await?;
        info!("Shut down {}", &name);
    }

    for name in statefulsets.keys() {
        sfs.patch_scale(
            name,
            &PatchParams::default(),
            &Patch::Merge(&json!({"spec": {"replicas": 0}})),
        )
        .await?;
        info!("Shut down {}", &name);
    }

    Ok(RestartableApps {
        deployments,
        statefulsets,
    })
}

#[tracing::instrument(skip(kube, restartable), err)]
async fn start_apps(
    kube: &kube::Client,
    restartable: &RestartableApps,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting apps");

    let deploy: Api<Deployment> = Api::default_namespaced(kube.clone());
    let sfs: Api<StatefulSet> = Api::default_namespaced(kube.clone());
    for (name, replicas) in &restartable.deployments {
        deploy
            .patch_scale(
                name,
                &PatchParams::default(),
                &Patch::Merge(&json!({"spec": {"replicas": replicas}})),
            )
            .await?;
        info!("Started {}", &name);
    }

    for (name, replicas) in &restartable.statefulsets {
        sfs.patch_scale(
            name,
            &PatchParams::default(),
            &Patch::Merge(&json!({"spec": {"replicas": replicas}})),
        )
        .await?;
        info!("Started {}", &name);
    }

    Ok(())
}

#[tracing::instrument(skip(kube), err)]
async fn find_newest_backup(kube: &kube::Client) -> Result<Backup, Box<dyn std::error::Error>> {
    let zeebe_backups = zeebe::list_backups(kube).await?;
    let operate_backups = operate::list_backups(kube).await?;
    let backup_id = list::find_most_recent_usable(&zeebe_backups, &operate_backups)
        .ok_or("No usable backup found")?;
    let zeebe_snapshot = format!("camunda_zeebe_records_{backup_id}");

    let operate_snapshots = operate::query_backup(kube, backup_id)
        .await?
        .details
        .iter()
        .map(|d| d.snapshot_name.clone())
        .collect::<Vec<String>>();

    info!("Using backup {}", backup_id);
    Ok(Backup {
        id: backup_id,
        snapshots: vec![zeebe_snapshot]
            .into_iter()
            .chain(operate_snapshots.into_iter())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RestoreTarget;

    #[test]
    fn test_determine_restore_target_auto() {
        let target = determine_restore_target(None, None).unwrap();
        assert!(matches!(target, RestoreTarget::RdbmsAuto));
    }

    #[test]
    fn test_determine_restore_target_backup_id() {
        let target = determine_restore_target(None, Some(123)).unwrap();
        assert!(matches!(target, RestoreTarget::RdbmsBackupId { id: 123 }));
    }

    #[test]
    fn test_determine_restore_target_point_in_time() {
        let target = determine_restore_target(Some("2024-01-01T12:00:00Z".into()), None).unwrap();
        match target {
            RestoreTarget::RdbmsPointInTime { to } => assert_eq!(to, "2024-01-01T12:00:00Z"),
            _ => panic!("Expected RdbmsPointInTime"),
        }
    }

    #[test]
    fn test_determine_restore_target_both_is_error() {
        let result = determine_restore_target(Some("ts".into()), Some(123));
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_args_for_rdbms_auto() {
        let target = RestoreTarget::RdbmsAuto;
        let args = restore_args_for_target(&target);
        assert!(args.is_empty());
    }

    #[test]
    fn test_restore_args_for_rdbms_backup_id() {
        let target = RestoreTarget::RdbmsBackupId { id: 42 };
        let args = restore_args_for_target(&target);
        assert_eq!(args, vec!["--backupId=42"]);
    }

    #[test]
    fn test_restore_args_for_rdbms_point_in_time() {
        let target = RestoreTarget::RdbmsPointInTime {
            to: "2024-01-01T00:00:00Z".into(),
        };
        let args = restore_args_for_target(&target);
        assert_eq!(args, vec!["--to=2024-01-01T00:00:00Z"]);
    }
}
