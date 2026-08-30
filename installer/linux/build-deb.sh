#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:?usage: build-deb.sh <version>}"
SRC="${2:-target/release/libomt_obs_plugin.so}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${ROOT}/release-assets"
STAGE="$(mktemp -d)"
trap 'rm -rf "${STAGE}"' EXIT

if [[ -f "${SRC}" ]]; then
  SO="${SRC}"
else
  SO="${ROOT}/${SRC}"
fi
test -f "${SO}"

install -D -m 755 "${SO}" \
  "${STAGE}/usr/lib/x86_64-linux-gnu/obs-plugins/libomt_obs_plugin.so"
install -D -m 644 "${ROOT}/LICENSE" \
  "${STAGE}/usr/share/doc/omt-obs-plugin/copyright"

mkdir -p "${STAGE}/DEBIAN"
cat > "${STAGE}/DEBIAN/control" <<EOF
Package: omt-obs-plugin
Version: ${VERSION}
Section: video
Priority: optional
Architecture: amd64
Maintainer: MikanseiLaboratory <https://github.com/MikanseiLaboratory/omt-obs-plugin>
Depends: obs-studio
Homepage: https://github.com/MikanseiLaboratory/omt-obs-plugin
Description: Pure-Rust Open Media Transport plugin for OBS Studio
 Coexists with the official C# omtplugin
 (https://github.com/openmediatransport/omtplugin).
EOF

mkdir -p "${OUT_DIR}"
DEB="${OUT_DIR}/omt-obs-plugin_${VERSION}_amd64.deb"
dpkg-deb --build "${STAGE}" "${DEB}"
echo "built ${DEB}"
