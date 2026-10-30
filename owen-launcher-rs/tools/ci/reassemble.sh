#!/usr/bin/env bash
# tools/ci/reassemble.sh — rebuild dist/game/fortnite-12.41.apk from the
# split repo parts (GitHub blocks files >100MB) and verify the checksums.
set -euo pipefail
cd "$(dirname "$0")/../../dist/game"
OUT="${1:-fortnite-12.41.apk}"
cat fortnite-12.41.apk.00 fortnite-12.41.apk.01 > "$OUT"
# verify both parts + the reassembled whole against SHA256SUMS
sha256sum -c <(rg 'fortnite-12.41.apk.0' SHA256SUMS) >/dev/null
REAL=$(sha256sum "$OUT" | cut -d' ' -f1)
EXPECTED=$(rg 'reassembled' SHA256SUMS | cut -d' ' -f1)
if [ "$REAL" != "$EXPECTED" ]; then
  echo "reassembled APK hash mismatch: $REAL != $EXPECTED" >&2
  exit 1
fi
echo "OK $OUT ($REAL)"
