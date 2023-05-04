use std::{
    collections::{BTreeSet, HashMap},
    error::Error,
};

use chrono::Utc;
use chrono::{LocalResult, TimeZone};

use chrono_humanize::HumanTime;
use tracing::{info, warn};

use crate::{
    operate,
    types::{BackupDescriptor, BackupState, OperateDetails, ZeebeDetails},
    zeebe,
};

#[tracing::instrument(err)]
pub(crate) async fn list() -> Result<(), Box<dyn Error>> {
    let kube = kube::Client::try_default().await?;
    let zeebe_backups: Vec<BackupDescriptor<crate::types::ZeebeDetails>> =
        zeebe::list_backups(&kube).await?;
    let operate_backups = operate::list_backups(&kube).await?;

    tracing::info_span!("Zeebe").in_scope(|| {
        print_backup_stats(&zeebe_backups);
    });
    tracing::info_span!("Operate").in_scope(|| {
        print_backup_stats(&operate_backups);
    });

    match find_most_recent_usable(&zeebe_backups, &operate_backups) {
        Some(backup_id) => {
            info!("The most recent usable backup is {}", backup_id);
            match Utc.timestamp_opt(backup_id as i64, 0) {
                LocalResult::Single(date) => {
                    info!(
                        "This backup was created {} at {}",
                        HumanTime::from(date),
                        date
                    );
                }
                _ => (),
            }
        }
        None => {
            warn!("No usable backups found");
        }
    }

    Ok(())
}

#[tracing::instrument(level = "debug")]
pub fn find_most_recent_usable(
    zeebe: &Vec<BackupDescriptor<ZeebeDetails>>,
    operate: &Vec<BackupDescriptor<OperateDetails>>,
) -> Option<u64> {
    let zeebe: BTreeSet<u64> = zeebe
        .into_iter()
        .filter(|b| b.state == BackupState::Completed)
        .map(|d| d.backup_id)
        .collect();
    let operate: BTreeSet<u64> = operate
        .into_iter()
        .filter(|b| b.state == BackupState::Completed)
        .map(|d| d.backup_id)
        .collect();

    zeebe.intersection(&operate).last().copied()
}

fn print_backup_stats<T>(backups: &Vec<BackupDescriptor<T>>) {
    let backups_by_state = backups.iter().fold(
        HashMap::<BackupState, Vec<&BackupDescriptor<T>>>::new(),
        |mut map, backup| {
            map.entry(backup.state).or_default().push(backup);
            map
        },
    );

    for (state, mut backups) in backups_by_state {
        backups.sort_by_key(|d| d.backup_id);
        let most_recent = backups
            .iter()
            .rev()
            .take(3)
            .map(|d| d.backup_id.to_string())
            .collect::<Vec<_>>();
        info!(
            "{} backups {:?}: {}, ...",
            backups.len(),
            state,
            most_recent.join(", ")
        );
    }
}
