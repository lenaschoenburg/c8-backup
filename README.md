# `c8-backup`

A CLI tool to automate backup and restore of Camunda Platform 8 deployments.

## Usage
> **Warning**
> This tool is experimental and not safe for use yet! Running it can result in complete dataloss.

Download a pre-built binary from the [latest release](https://github.com/oleschoenburg/c8-backup/releases) or build from sources with
```shell
cargo install c8-backup
```

Running `c8-backup` connects to the current cluster and namespace based on your kubernetes context.
The restore task starts without any additional confirmation and will take the following steps:
1. Find the latest backup from Zeebe and Operate
2. Stop Zeebe and Operate
3. Delete **all** indices from Elasticsearch
4. Restore Elasticsearch indices based on backups
5. Delete all Zeebe data
6. Restore Zeebe data based on backup
7. Start Zeebe and Operate

## Current Status

Features:
- [ ] Take backups
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