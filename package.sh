#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PARENT="$(dirname "$ROOT")"
NAME="$(basename "$ROOT")"
ZIP="$PARENT/$NAME.zip"
chmod +x "$ROOT"/*.sh
rm -f "$ZIP" "$ZIP.sha256"
cd "$PARENT"
zip -r "$ZIP" "$NAME" -x "$NAME/target/*" "$NAME/.git/*" "$NAME/*.log" "$NAME/*.webm" "$NAME/*.mp4" "$NAME/*.opus" "$NAME/*.m4a"
sha256sum "$ZIP" > "$ZIP.sha256"
echo "Criado: $ZIP"
echo "Checksum: $ZIP.sha256"
