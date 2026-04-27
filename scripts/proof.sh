#!/usr/bin/env bash
set -euo pipefail

NS="${NS:-vuejs}"
OUT="${OUT:-proof.txt}"

{
  echo "===== $(date -Iseconds) ====="
  echo
  echo "### kubectl get nodes -o wide"
  kubectl get nodes -o wide
  echo
  echo "### kubectl -n $NS get all"
  kubectl -n "$NS" get all
  echo
  echo "### pods -o wide (répartition noeuds)"
  kubectl -n "$NS" get pods -o wide
  echo
  echo "### hpa"
  kubectl -n "$NS" get hpa
  echo
  echo "### pdb"
  kubectl -n "$NS" get pdb
  echo
  echo "### ingress"
  kubectl -n "$NS" get ingress
  echo
  echo "### describe deploy (head)"
  kubectl -n "$NS" describe deploy vuejs-template | sed -n '1,80p'
  echo
  echo "### securityContext (preuve non-root)"
  kubectl -n "$NS" get pods -o jsonpath='{range .items[*]}{.metadata.name}{"  user="}{.spec.containers[0].securityContext.runAsUser}{"  nonRoot="}{.spec.securityContext.runAsNonRoot}{"  roFs="}{.spec.containers[0].securityContext.readOnlyRootFilesystem}{"  drop="}{.spec.containers[0].securityContext.capabilities.drop}{"\n"}{end}'
  echo
  echo "### exec id (preuve uid runtime)"
  kubectl -n "$NS" exec deploy/vuejs-template -- id
  echo
  echo "### probe HTTP via port-forward"
  kubectl -n "$NS" port-forward svc/vuejs-template 18080:80 >/tmp/pf.log 2>&1 &
  PF=$!
  sleep 2
  curl -sS -o /dev/null -w "HTTP %{http_code} svc/healthz\n" http://127.0.0.1:18080/healthz || true
  curl -sS -o /dev/null -w "HTTP %{http_code} svc/\n" http://127.0.0.1:18080/ || true
  kill $PF 2>/dev/null || true
  echo
  echo "### probe HTTP via ingress (host vuejs.localhost)"
  curl -sS -o /dev/null -w "HTTP %{http_code} ingress/healthz\n" -H 'Host: vuejs.localhost' http://127.0.0.1:8080/healthz || true
} | tee "$OUT"

echo
echo "preuve écrite: $OUT"
