#!/usr/bin/env bash
set -u
failures=0
pass(){ echo "[PASS] $*"; }
warn(){ echo "[AVISO] $*"; }
fail(){ echo "[FALHA] $*"; failures=$((failures+1)); }
for c in rustc cargo gcc pkg-config ffmpeg pactl xrandr; do command -v "$c" >/dev/null 2>&1 && pass "$c disponível" || fail "$c ausente"; done
for m in gtk4 libadwaita-1.0 gstreamer-1.0 libpipewire-0.3; do pkg-config --exists "$m" 2>/dev/null && pass "$m $(pkg-config --modversion "$m")" || fail "$m ausente"; done
[[ "${XDG_SESSION_TYPE:-}" == x11 ]] && pass "Sessão X11 compatível com a captura do MVP" || warn "A captura de tela do MVP requer X11"
systemctl --user is-active --quiet pipewire 2>/dev/null && pass "PipeWire ativo" || warn "PipeWire não está ativo"
echo "Falhas: $failures"
((failures==0))
