#!/usr/bin/env bash
set -euo pipefail

CLUSTER="${CLUSTER:-vuejs}"
IMAGE="${IMAGE:-ghcr.io/zuhowks/vuejs-template:latest}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo ">>> [1/5] cluster k3d ($CLUSTER)"
if ! k3d cluster list | awk 'NR>1 {print $1}' | grep -qx "$CLUSTER"; then
  k3d cluster create --config "$ROOT/k3d-config.yaml"
else
  echo "cluster $CLUSTER déjà présent"
fi

echo ">>> [2/5] build image locale"
docker build -t "$IMAGE" "$ROOT"

echo ">>> [3/5] import image dans k3d"
k3d image import "$IMAGE" -c "$CLUSTER"

echo ">>> [4/5] metrics-server (HPA)"
kubectl -n kube-system patch deploy metrics-server --type=json \
  -p='[{"op":"add","path":"/spec/template/spec/containers/0/args/-","value":"--kubelet-insecure-tls"}]' || true
kubectl -n kube-system rollout status deploy/metrics-server --timeout=120s || true

echo ">>> [5/5] apply manifests"
kubectl apply -f "$ROOT/k8s/"
kubectl -n vuejs rollout status deploy/vuejs-template --timeout=180s

echo
echo "OK. Preuve: scripts/proof.sh"
