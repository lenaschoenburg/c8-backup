# `c8-backup`

A CLI tool to automate backup and restore of Camunda Platform 8 deployments.

## Current Status

**⚠️ Experimental ⚠️**

Features:
- [x] Take backups
- [x] Restore backups
- [ ] Dry runs

Components:
- [x] Zeebe
- [x] Operate
- [ ] Tasklist
- [ ] Optimize

Deployments:
- [x] Remote [Camunda Platform 8 Helm] installation (running locally, connecting through the current kubernetes context).
- [ ] Local [Camunda Platform 8 Helm] installation (running as a pod inside the cluster).


[Camunda Platform 8 Helm]: https://github.com/camunda/camunda-platform-helm

## Usage
> **Warning**
> This tool is experimental and not safe for use yet! Running it can result in complete dataloss.

Download a pre-built binary from the [latest release](https://github.com/oleschoenburg/c8-backup/releases) or build from sources with
```shell
cargo install c8-backup
```

Currently, this tool is meant to run locally. It will connect to your current kubernetes context and tries to find C8 components there.

### Taking backups

The `backup` command starts without any additional confirmation and will take the following steps:
1. Take a backup of Operate
2. Pause Zeebe exporting
3. Take a backup of exported Zeebe records
4. Take a backup of Zeebe
5. Resume Zeebe exporting

Resuming exporting is crucial and this command tries to resume exporting if any error occurs while taking a backup 
but you should manually confirm that exporting resumed, for example by following the log output.


```shell
$ c8-backup backup
c8_backup::backup::backup{}
  c8_backup::backup::try_backup{new_backup=Backup { backup_id: "1683126300" }}
    c8_backup::backup::backup_operate{new_backup=Backup { backup_id: "1683126300" }}
      226ms  INFO c8_backup::backup Started backup
      495ms  INFO c8_backup::backup Checking again in 5 seconds, backup.state=InProgress
      5867ms  INFO c8_backup::backup Checking again in 5 seconds, backup.state=InProgress
      11262ms  INFO c8_backup::backup Checking again in 5 seconds, backup.state=InProgress
      16582ms  INFO c8_backup::backup Backup completed
    
    c8_backup::backup::pause_exporting{}
    
    c8_backup::backup::backup_zeebe_export{new_backup=Backup { backup_id: "1683126300" }}
    
    c8_backup::backup::backup_zeebe{new_backup=Backup { backup_id: "1683126300" }}
      331ms  INFO c8_backup::backup Started backup
      764ms  INFO c8_backup::backup Checking again in 5 seconds, backup.state=InProgress
      6232ms  INFO c8_backup::backup Backup completed
    
    c8_backup::backup::resume_exporting{}
```

### Restoring backups

The `restore` commands starts without any additional confirmation and will take the following steps:
1. Find the latest backup from Zeebe and Operate
2. Stop Zeebe and Operate
3. Delete **all** indices from Elasticsearch
4. Restore Elasticsearch indices based on backups
5. Delete all Zeebe data
6. Restore Zeebe data based on backup
7. Start Zeebe and Operate

```shell
$ c8-backup restore
c8_backup::restore::restore{}
  c8_backup::restore::find_newest_backup{}
    1461ms  INFO c8_backup::restore Using backup 1683126300
  
  c8_backup::restore::shutdown_apps{}
    369ms  INFO c8_backup::restore Shut down ccs23-dev-operate
    424ms  INFO c8_backup::restore Shut down ccs23-dev-zeebe-gateway
    471ms  INFO c8_backup::restore Shut down ccs23-dev-zeebe
  
  c8_backup::restore::delete_indices{}
    534ms  INFO c8_backup::restore Deleted index operate-incident-8.2.0_
    861ms  INFO c8_backup::restore Deleted index operate-import-position-8.2.0_
    1189ms  INFO c8_backup::restore Deleted index operate-batch-operation-1.0.0_
    1443ms  INFO c8_backup::restore Deleted index operate-event-8.1.0_
    1731ms  INFO c8_backup::restore Deleted index operate-decision-1.0.0_
    2026ms  INFO c8_backup::restore Deleted index operate-operation-8.2.0_
    2284ms  INFO c8_backup::restore Deleted index operate-migration-steps-repository-1.1.0_
    2607ms  INFO c8_backup::restore Deleted index operate-variable-8.2.0_
    2913ms  INFO c8_backup::restore Deleted index operate-web-session-1.1.0_
    3242ms  INFO c8_backup::restore Deleted index operate-decision-instance-8.2.0_
    3509ms  INFO c8_backup::restore Deleted index operate-decision-requirements-1.0.0_
    3825ms  INFO c8_backup::restore Deleted index operate-process-8.1.8_
    4112ms  INFO c8_backup::restore Deleted index operate-list-view-8.1.0_
    4453ms  INFO c8_backup::restore Deleted index operate-flownode-instance-8.2.0_
    4806ms  INFO c8_backup::restore Deleted index operate-sequence-flow-8.2.0_
    5074ms  INFO c8_backup::restore Deleted index operate-metric-1.0.0_
    5381ms  INFO c8_backup::restore Deleted index operate-user-1.2.0_
  
  c8_backup::restore::restore_indices{backup=Backup { id: 1683126300, snapshots: ["camunda_zeebe_records_1683126300", "camunda_operate_1683126300_8.2.3_part_1_of_6", "camunda_operate_1683126300_8.2.3_part_2_of_6", "camunda_operate_1683126300_8.2.3_part_3_of_6", "camunda_operate_1683126300_8.2.3_part_4_of_6", "camunda_operate_1683126300_8.2.3_part_5_of_6", "camunda_operate_1683126300_8.2.3_part_6_of_6"] }}
    20316ms  INFO c8_backup::restore Restored snapshot camunda_zeebe_records_1683126300
    22038ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683126300_8.2.3_part_1_of_6
    27386ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683126300_8.2.3_part_2_of_6
    27915ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683126300_8.2.3_part_3_of_6
    37721ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683126300_8.2.3_part_4_of_6
    38154ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683126300_8.2.3_part_5_of_6
    41046ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683126300_8.2.3_part_6_of_6
  
  c8_backup::restore::delete_zeebe_data{backup=Backup { id: 1683126300, snapshots: ["camunda_zeebe_records_1683126300", "camunda_operate_1683126300_8.2.3_part_1_of_6", "camunda_operate_1683126300_8.2.3_part_2_of_6", "camunda_operate_1683126300_8.2.3_part_3_of_6", "camunda_operate_1683126300_8.2.3_part_4_of_6", "camunda_operate_1683126300_8.2.3_part_5_of_6", "camunda_operate_1683126300_8.2.3_part_6_of_6"] }}
    266ms  INFO c8_backup::restore Deleting data of data-ccs23-dev-zeebe-0
    293ms  INFO c8_backup::restore Deleting data of data-ccs23-dev-zeebe-1
    323ms  INFO c8_backup::restore Deleting data of data-ccs23-dev-zeebe-2
    14185ms  INFO c8_backup::restore Deleted data of data-ccs23-dev-zeebe-0
    16215ms  INFO c8_backup::restore Deleted data of data-ccs23-dev-zeebe-1
    16318ms  INFO c8_backup::restore Deleted data of data-ccs23-dev-zeebe-2
  
  c8_backup::restore::restore_zeebe_data{backup=Backup { id: 1683126300, snapshots: ["camunda_zeebe_records_1683126300", "camunda_operate_1683126300_8.2.3_part_1_of_6", "camunda_operate_1683126300_8.2.3_part_2_of_6", "camunda_operate_1683126300_8.2.3_part_3_of_6", "camunda_operate_1683126300_8.2.3_part_4_of_6", "camunda_operate_1683126300_8.2.3_part_5_of_6", "camunda_operate_1683126300_8.2.3_part_6_of_6"] }}
    131ms  INFO c8_backup::restore Restoring data of data-ccs23-dev-zeebe-0
    162ms  INFO c8_backup::restore Restoring data of data-ccs23-dev-zeebe-1
    194ms  INFO c8_backup::restore Restoring data of data-ccs23-dev-zeebe-2
    22045ms  INFO c8_backup::restore Restored data of data-ccs23-dev-zeebe-0
    23196ms  INFO c8_backup::restore Restored data of data-ccs23-dev-zeebe-1
    23295ms  INFO c8_backup::restore Restored data of data-ccs23-dev-zeebe-2
  
  c8_backup::restore::start_apps{}
    0ms  INFO c8_backup::restore Starting apps
    27ms  INFO c8_backup::restore Started ccs23-dev-operate
    53ms  INFO c8_backup::restore Started ccs23-dev-zeebe-gateway
    83ms  INFO c8_backup::restore Started ccs23-dev-zeebe
```
