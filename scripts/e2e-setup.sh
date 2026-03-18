#!/usr/bin/env bash
# Sets up a k3d cluster with minimal Camunda 8 for e2e testing.
# Usage: ./scripts/e2e-setup.sh [es|rdbms]
# Cleanup: ./scripts/e2e-teardown.sh
set -euo pipefail

MODE="${1:-es}"
CLUSTER_NAME="${C8_BACKUP_TEST_CLUSTER:-c8-backup-test}"
NAMESPACE="${C8_BACKUP_TEST_NAMESPACE:-camunda}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$MODE" = "rdbms" ]; then
  VALUES_FILE="$SCRIPT_DIR/../tests/e2e/rdbms-values.yaml"
else
  VALUES_FILE="$SCRIPT_DIR/../tests/e2e/test-values.yaml"
fi

echo "==> Creating k3d cluster '$CLUSTER_NAME' (mode: $MODE)..."
k3d cluster create "$CLUSTER_NAME" \
  --agents 1 \
  --wait \
  --k3s-arg "--disable=traefik@server:0" \
  --timeout 120s

echo "==> Adding Camunda Helm repo..."
helm repo add camunda https://helm.camunda.io 2>/dev/null || true
helm repo update camunda

echo "==> Installing Camunda 8 ($MODE mode)..."
helm install camunda camunda/camunda-platform \
  --namespace "$NAMESPACE" \
  --create-namespace \
  --values "$VALUES_FILE" \
  --timeout 10m \
  --wait

echo "==> Waiting for pods to be ready..."
kubectl wait --for=condition=ready pod \
  -l app.kubernetes.io/part-of=camunda-platform \
  -n "$NAMESPACE" \
  --timeout=300s || true

kubectl get pods -n "$NAMESPACE"
echo ""
echo "Run: cargo test --test e2e_${MODE} -- --nocapture --ignored"
echo "Cleanup: ./scripts/e2e-teardown.sh"
