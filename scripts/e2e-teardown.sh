#!/usr/bin/env bash
set -euo pipefail
CLUSTER_NAME="${C8_BACKUP_TEST_CLUSTER:-c8-backup-test}"
echo "==> Deleting k3d cluster '$CLUSTER_NAME'..."
k3d cluster delete "$CLUSTER_NAME"
echo "==> Done."
