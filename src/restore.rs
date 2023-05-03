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
    operate,
    types::BackupState,
    zeebe,
};

#[tracing::instrument(err)]
pub(crate) async fn restore() -> Result<(), Box<dyn std::error::Error>> {
    let kube = kube::Client::try_default().await?;
    let backup = find_newest_backup(&kube).await?;
    let restartable = shutdown_apps(&kube).await?;

    delete_indices(&kube).await?;
    restore_indices(&kube, &backup).await?;

    delete_zeebe_data(&kube, &backup).await?;
    restore_zeebe_data(&kube, &backup).await?;

    start_apps(&kube, &restartable).await?;

    Ok(())
}

#[derive(Debug)]
struct Backup {
    id: u64,
    snapshots: Vec<String>,
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
    backup: &Backup,
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
                        command: Some(vec![
                            "/usr/local/zeebe/bin/restore".to_string(),
                            format!("--backupId={}", backup.id),
                        ]),
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
async fn delete_zeebe_data(
    kube: &kube::Client,
    backup: &Backup,
) -> Result<(), Box<dyn std::error::Error>> {
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
async fn restore_zeebe_data(
    kube: &kube::Client,
    backup: &Backup,
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
        let job = zeebe_data_restoration_job(backup, pvc, &zeebe);
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

    let completed = zeebe_backups
        .iter()
        .filter(|d| d.state == BackupState::Completed)
        .max_by_key(|d| d.backup_id)
        .ok_or("No completed backup found")?;

    let id = completed.backup_id;
    let zeebe_snapshot = format!("camunda_zeebe_records_{id}");

    let operate_backup = operate::query_backup(kube, id).await?;
    let operate_snapshots = operate_backup
        .details
        .iter()
        .map(|d| d.snapshot_name.clone())
        .collect::<Vec<String>>();

    info!("Using backup {}", id);
    Ok(Backup {
        id: completed.backup_id,
        snapshots: vec![zeebe_snapshot]
            .into_iter()
            .chain(operate_snapshots.into_iter())
            .collect(),
    })
}
