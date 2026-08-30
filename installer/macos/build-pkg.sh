#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?usage: build-pkg.sh <version>}"
SRC="${2:-target/release/libomt_obs_plugin.dylib}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${ROOT}/release-assets"
BUNDLE_NAME="omt-obs-plugin.plugin"
PAYLOAD="$(mktemp -d)"
trap 'rm -rf "${PAYLOAD}"' EXIT

test -f "${ROOT}/${SRC}" || test -f "${SRC}"
if [[ -f "${SRC}" ]]; then
  DYLIB="${SRC}"
else
  DYLIB="${ROOT}/${SRC}"
fi

CONTENTS="${PAYLOAD}/Library/Application Support/obs-studio/plugins/${BUNDLE_NAME}/Contents"
mkdir -p "${CONTENTS}/MacOS" "${CONTENTS}/Resources"
cp "${DYLIB}" "${CONTENTS}/MacOS/libomt_obs_plugin.dylib"
sed "s/0.1.0/${VERSION}/g" "${ROOT}/installer/macos/Info.plist" > "${CONTENTS}/Info.plist"
cp "${ROOT}/LICENSE" "${CONTENTS}/Resources/LICENSE.txt"

mkdir -p "${OUT_DIR}"
pkgbuild \
  --root "${PAYLOAD}" \
  --identifier lab.mikansei.omt-obs-plugin \
  --version "${VERSION}" \
  --install-location / \
  "${OUT_DIR}/omt-obs-plugin-${VERSION}-macos-arm64.pkg"

echo "built ${OUT_DIR}/omt-obs-plugin-${VERSION}-macos-arm64.pkg"
