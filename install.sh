#!/usr/bin/env bash
set -Eeuo pipefail
APP_NAME="Gravador Linux"
APP_BIN="gravador-linux"
APP_ID="br.com.leo.GravadorLinux"
ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
LOG_DIR="${HOME}/.local/state/gravador-linux-installer"
LOG_FILE="${LOG_DIR}/install-$(date +%Y%m%d-%H%M%S).log"
YES=0
DRY_RUN=0
SKIP_APT_UPDATE=0
mkdir -p "$LOG_DIR"
exec > >(tee -a "$LOG_FILE") 2>&1
trap 'printf "\n[ERRO] Falha na linha %s. Consulte: %s\n" "$LINENO" "$LOG_FILE"' ERR
usage(){ echo "Uso: ./install.sh [--yes] [--dry-run] [--skip-apt-update] [--help]"; }
for arg in "$@"; do case "$arg" in --yes) YES=1;; --dry-run) DRY_RUN=1;; --skip-apt-update) SKIP_APT_UPDATE=1;; --help) usage; exit 0;; *) echo "Opção desconhecida: $arg"; exit 2;; esac; done
run(){ printf '  $'; printf ' %q' "$@"; echo; ((DRY_RUN)) || "$@"; }
confirm(){ ((YES)) && return 0; read -r -p "$1 [s/N]: " a; [[ "${a,,}" == "s" || "${a,,}" == "sim" ]]; }
[[ $EUID -ne 0 ]] || { echo "Não execute o instalador inteiro como root."; exit 1; }
source /etc/os-release
[[ "${ID:-}" == "linuxmint" || "${ID_LIKE:-}" == *ubuntu* || "${ID_LIKE:-}" == *debian* ]] || { echo "Sistema não compatível."; exit 1; }
command -v sudo >/dev/null || { echo "sudo não encontrado."; exit 1; }
sudo -v
packages=(build-essential pkg-config git curl ca-certificates cmake meson ninja-build libgtk-4-dev libadwaita-1-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-tools gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav gstreamer1.0-pipewire pipewire pipewire-audio libpipewire-0.3-dev libspa-0.2-dev xdg-desktop-portal xdg-desktop-portal-gtk ffmpeg pulseaudio-utils x11-xserver-utils desktop-file-utils zip)
missing=()
echo "Sistema: ${PRETTY_NAME:-desconhecido} | Sessão: ${XDG_SESSION_TYPE:-desconhecida}"
echo "[1/6] Verificando dependências..."
for p in "${packages[@]}"; do
  if dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q 'install ok installed'; then echo "[PULAR] $p";
  elif apt-cache show "$p" >/dev/null 2>&1; then missing+=("$p");
  else echo "[AVISO] Pacote indisponível: $p"; fi
done
if ((${#missing[@]})); then
  printf 'Pacotes ausentes:\n'; printf ' - %s\n' "${missing[@]}"
  confirm "Instalar os pacotes ausentes?" || exit 1
  ((SKIP_APT_UPDATE)) || run sudo apt-get update
  run sudo apt-get install -y "${missing[@]}"
else echo "Nenhum pacote APT precisa ser instalado."; fi

echo "[2/6] Verificando Rust e Cargo..."
if command -v rustc >/dev/null && command -v cargo >/dev/null; then rustc --version; cargo --version;
else
  confirm "Instalar Rust e Cargo pelo rustup oficial?" || exit 1
  if ((DRY_RUN)); then echo "[DRY-RUN] Instalação do rustup"; else tmp=$(mktemp); curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$tmp"; sh "$tmp" -y --profile minimal --default-toolchain stable; rm -f "$tmp"; source "$HOME/.cargo/env"; rustup component add rustfmt clippy; fi
fi
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

echo "[3/6] Compilando..."
cd "$ROOT_DIR"
run cargo build --release

echo "[4/6] Instalando para o usuário atual..."
if ((!DRY_RUN)); then
  install -d "$HOME/.local/bin" "$HOME/.local/share/applications" "$HOME/.local/share/icons/hicolor/scalable/apps"
  install -m 0755 "target/release/$APP_BIN" "$HOME/.local/bin/$APP_BIN"
  sed "s|@HOME@|$HOME|g" packaging/gravador-linux.desktop.in > "$HOME/.local/share/applications/$APP_ID.desktop"
  install -m 0644 assets/gravador-linux.svg "$HOME/.local/share/icons/hicolor/scalable/apps/$APP_ID.svg"
  update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true
fi

echo "[5/6] Verificando ambiente..."
((DRY_RUN)) || "$ROOT_DIR/verify-environment.sh"
echo "[6/6] Instalação concluída. Log: $LOG_FILE"
if ((!DRY_RUN)) && confirm "Abrir o aplicativo agora?"; then nohup "$HOME/.local/bin/$APP_BIN" >/dev/null 2>&1 & fi
