#!/usr/bin/env bash
set -euxo pipefail
context=.cache/qa-container
mkdir -p "$context"
cp scripts/qa-linux/Dockerfile "$context/Dockerfile"
cp -a "$(dirname "$(readlink -f "$(command -v pwsh)")")" "$context/powershell"
trap 'docker rm -f qa-tools-22.04 qa-tools-24.04 >/dev/null 2>&1 || true' EXIT
for version in 22.04 24.04; do
  docker build --build-arg "BASE=ubuntu:$version" -t "yoyo-qa:$version" "$context"
  timeout 45s docker run --rm --name "qa-tools-$version" --entrypoint /bin/bash "yoyo-qa:$version" -c 'echo CONTAINER_READY; if ldconfig -p | grep libmpv; then exit 1; fi; echo NO_SYSTEM_MPV'
  timeout 45s docker run --rm --name "qa-tools-$version" --entrypoint /usr/local/bin/pwsh "yoyo-qa:$version" -NoLogo -NoProfile -Command '$PSVersionTable.PSVersion; Write-Output POWERSHELL_READY'
done
