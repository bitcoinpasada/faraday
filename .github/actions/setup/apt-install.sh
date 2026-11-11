#!/usr/bin/env bash
# Installs the named Ubuntu packages for the check (docs/decisions/F183.md).
# A mirror can stall with no error, so each try has a time limit, or the job
# would wait out its own timeout. When a try fails, apt is pointed at a
# second public Ubuntu mirror for the tries after it: a runner can fail to
# look up Ubuntu's own archive hosts while Ubuntu's servers are up (seen
# 2026-11-11). Every mirror serves the same archive, and apt still checks it
# against Ubuntu's signing keys, so only where the packages come from
# changes, never what is installed.
set -uo pipefail
FALLBACK=https://mirrors.edge.kernel.org/ubuntu/
apt() { timeout "$1" sudo apt-get -o Acquire::Retries=2 -o Acquire::http::Timeout=30 "${@:2}"; }
for try in 1 2 3; do
  apt 240 update && apt 300 install --yes --no-install-recommends "$@" && exit 0
  echo "apt: try $try did not finish; the next uses $FALLBACK"
  for f in /etc/apt/sources.list /etc/apt/sources.list.d/*.sources /etc/apt/sources.list.d/*.list; do
    [ -f "$f" ] || continue
    sudo sed -E -i \
      -e "s#mirror\+file:/etc/apt/apt-mirrors.txt/?#$FALLBACK#g" \
      -e "s#https?://([a-z0-9-]+\.)?(archive|security)\.ubuntu\.com/ubuntu/?#$FALLBACK#g" "$f"
  done
done
exit 1
