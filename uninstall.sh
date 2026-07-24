#!/usr/bin/env bash
set -Eeuo pipefail
APP_ID="br.com.leo.GravadorLinux"
read -r -p "Remover o Gravador Linux? [s/N]: " a
if [[ "${a,,}" == s || "${a,,}" == sim ]]; then
  rm -f "$HOME/.local/bin/gravador-linux" "$HOME/.local/share/applications/$APP_ID.desktop" "$HOME/.local/share/icons/hicolor/scalable/apps/$APP_ID.svg"
  update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
  echo "Aplicação removida. Dependências do sistema foram preservadas."
fi
