# `c8-backup`

A CLI tool to automate backup and restore of Camunda Platform 8 deployments.

## Current Status

**⚠️ Experimental ⚠️**

Features:
- [x] List backups
- [x] Create backups
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

### Listing backups

The `list` command lists recent backups by status and also shows which is the most recent, usable, backup.
This means that the backup is successfully completed by all components.
```
$ c8-backup list
c8_backup::list::list{}
  c8_backup::list::Zeebe{}
    0ms  INFO c8_backup::list 2 backups Completed: 1683214620, 1683214072, ...
  
  c8_backup::list::Operate{}
    0ms  INFO c8_backup::list 2 backups Completed: 1683214620, 1683214072, ...
  
  1060ms  INFO c8_backup::list The most recent usable backup is 1683214620
  1060ms  INFO c8_backup::list This backup was created 8 minutes ago at 2023-05-04 15:37:00 UTC
```

### Creating backups

The `create` command starts without any additional confirmation and will take the following steps:
1. Take a backup of Operate
2. Pause Zeebe exporting
3. Take a backup of exported Zeebe records
4. Take a backup of Zeebe
5. Resume Zeebe exporting

Resuming exporting is crucial and this command tries to resume exporting if any error occurs while taking a backup 
but you should manually confirm that exporting resumed, for example by following the log output.


```shell
$ c8-backup create
c8_backup::create::create{}
  c8_backup::create::try_backup{backup_id=1683214620}
    c8_backup::create::backup_operate{backup_id=1683214620}
      c8_backup::operate::take_backup{backup_id=1683214620}
      
      267ms  INFO c8_backup::create Started backup
      521ms  INFO c8_backup::create Checking again in 5 seconds, state is InProgress
      5870ms  INFO c8_backup::create Checking again in 5 seconds, state is InProgress
      11233ms  INFO c8_backup::create Checking again in 5 seconds, state is Incomplete
      16598ms  INFO c8_backup::create Backup completed
    
    c8_backup::zeebe::pause_exporting{}
    
    c8_backup::create::backup_zeebe_export{backup_id=1683214620}
      c8_backup::elasticsearch::take_snapshot{req=SnapshotRequest { indices: "zeebe-record*", feature_states: ["none"] }, name="camunda_zeebe_records_1683214620"}
      
    
    c8_backup::create::backup_zeebe{backup_id=1683214620}
      c8_backup::zeebe::take_backup{backup_id=1683214620}
      
      280ms  INFO c8_backup::create Started backup
      831ms  INFO c8_backup::create Checking again in 5 seconds, state is InProgress
      6180ms  INFO c8_backup::create Backup completed
    
    c8_backup::zeebe::resume_exporting{}```
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
    1573ms  INFO c8_backup::restore Using backup 1683214620
  
  c8_backup::restore::shutdown_apps{}
    331ms  INFO c8_backup::restore Shut down ccs23-dev-zeebe-gateway
    380ms  INFO c8_backup::restore Shut down ccs23-dev-operate
    431ms  INFO c8_backup::restore Shut down ccs23-dev-zeebe
  
  c8_backup::restore::delete_indices{}
    c8_backup::elasticsearch::get_all_indices{}
    
    434ms  INFO c8_backup::elasticsearch Deleted index operate-flownode-instance-8.2.0_
    695ms  INFO c8_backup::elasticsearch Deleted index operate-decision-instance-8.2.0_
    966ms  INFO c8_backup::elasticsearch Deleted index operate-list-view-8.1.0_
    1263ms  INFO c8_backup::elasticsearch Deleted index operate-import-position-8.2.0_
    1587ms  INFO c8_backup::elasticsearch Deleted index operate-user-1.2.0_
    1907ms  INFO c8_backup::elasticsearch Deleted index zeebe-record_deployment-distribution_8.2.3_2023-05-04
    2223ms  INFO c8_backup::elasticsearch Deleted index operate-operation-8.2.0_
    2509ms  INFO c8_backup::elasticsearch Deleted index zeebe-record_deployment_8.2.3_2023-05-04
    2809ms  INFO c8_backup::elasticsearch Deleted index operate-batch-operation-1.0.0_
    3077ms  INFO c8_backup::elasticsearch Deleted index operate-process-8.1.8_
    3352ms  INFO c8_backup::elasticsearch Deleted index operate-web-session-1.1.0_
    3634ms  INFO c8_backup::elasticsearch Deleted index operate-incident-8.2.0_
    3938ms  INFO c8_backup::elasticsearch Deleted index zeebe-record_variable_8.2.3_2023-05-04
    4216ms  INFO c8_backup::elasticsearch Deleted index zeebe-record_job_8.2.3_2023-05-04
    4472ms  INFO c8_backup::elasticsearch Deleted index operate-variable-8.2.0_
    4796ms  INFO c8_backup::elasticsearch Deleted index operate-migration-steps-repository-1.1.0_
    5069ms  INFO c8_backup::elasticsearch Deleted index operate-decision-1.0.0_
    5324ms  INFO c8_backup::elasticsearch Deleted index operate-event-8.1.0_
    5624ms  INFO c8_backup::elasticsearch Deleted index zeebe-record_process-instance-creation_8.2.3_2023-05-04
    5939ms  INFO c8_backup::elasticsearch Deleted index zeebe-record_process_8.2.3_2023-05-04
    6234ms  INFO c8_backup::elasticsearch Deleted index operate-metric-1.0.0_
    6526ms  INFO c8_backup::elasticsearch Deleted index operate-sequence-flow-8.2.0_
    6844ms  INFO c8_backup::elasticsearch Deleted index zeebe-record_process-instance_8.2.3_2023-05-04
    7164ms  INFO c8_backup::elasticsearch Deleted index operate-decision-requirements-1.0.0_
  
  c8_backup::restore::restore_indices{backup=Backup { id: 1683214620, snapshots: ["camunda_zeebe_records_1683214620", "camunda_operate_1683214620_8.2.3_part_1_of_6", "camunda_operate_1683214620_8.2.3_part_2_of_6", "camunda_operate_1683214620_8.2.3_part_3_of_6", "camunda_operate_1683214620_8.2.3_part_4_of_6", "camunda_operate_1683214620_8.2.3_part_5_of_6", "camunda_operate_1683214620_8.2.3_part_6_of_6"] }}
    12203ms  INFO c8_backup::restore Restored snapshot camunda_zeebe_records_1683214620
    14050ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683214620_8.2.3_part_1_of_6
    18322ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683214620_8.2.3_part_2_of_6
    18925ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683214620_8.2.3_part_3_of_6
    23336ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683214620_8.2.3_part_4_of_6
    23991ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683214620_8.2.3_part_5_of_6
    26099ms  INFO c8_backup::restore Restored snapshot camunda_operate_1683214620_8.2.3_part_6_of_6
  
  c8_backup::restore::delete_zeebe_data{backup=Backup { id: 1683214620, snapshots: ["camunda_zeebe_records_1683214620", "camunda_operate_1683214620_8.2.3_part_1_of_6", "camunda_operate_1683214620_8.2.3_part_2_of_6", "camunda_operate_1683214620_8.2.3_part_3_of_6", "camunda_operate_1683214620_8.2.3_part_4_of_6", "camunda_operate_1683214620_8.2.3_part_5_of_6", "camunda_operate_1683214620_8.2.3_part_6_of_6"] }}
    303ms  INFO c8_backup::restore Deleting data of data-ccs23-dev-zeebe-0
    332ms  INFO c8_backup::restore Deleting data of data-ccs23-dev-zeebe-1
    360ms  INFO c8_backup::restore Deleting data of data-ccs23-dev-zeebe-2
    17284ms  INFO c8_backup::restore Deleted data of data-ccs23-dev-zeebe-0
    17385ms  INFO c8_backup::restore Deleted data of data-ccs23-dev-zeebe-1
    17482ms  INFO c8_backup::restore Deleted data of data-ccs23-dev-zeebe-2
  
  c8_backup::restore::restore_zeebe_data{backup=Backup { id: 1683214620, snapshots: ["camunda_zeebe_records_1683214620", "camunda_operate_1683214620_8.2.3_part_1_of_6", "camunda_operate_1683214620_8.2.3_part_2_of_6", "camunda_operate_1683214620_8.2.3_part_3_of_6", "camunda_operate_1683214620_8.2.3_part_4_of_6", "camunda_operate_1683214620_8.2.3_part_5_of_6", "camunda_operate_1683214620_8.2.3_part_6_of_6"] }}
    97ms  INFO c8_backup::restore Restoring data of data-ccs23-dev-zeebe-0
    131ms  INFO c8_backup::restore Restoring data of data-ccs23-dev-zeebe-1
    161ms  INFO c8_backup::restore Restoring data of data-ccs23-dev-zeebe-2
    26025ms  INFO c8_backup::restore Restored data of data-ccs23-dev-zeebe-0
    26117ms  INFO c8_backup::restore Restored data of data-ccs23-dev-zeebe-1
    26210ms  INFO c8_backup::restore Restored data of data-ccs23-dev-zeebe-2
  
  c8_backup::restore::start_apps{}
    0ms  INFO c8_backup::restore Starting apps
    29ms  INFO c8_backup::restore Started ccs23-dev-zeebe-gateway
    59ms  INFO c8_backup::restore Started ccs23-dev-operate
    90ms  INFO c8_backup::restore Started ccs23-dev-zeebe
 ```
