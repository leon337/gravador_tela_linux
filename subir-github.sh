#!/usr/bin/env bash
set -Eeuo pipefail

# ============================================================
# Publica o projeto local "gravador-linux-mvp" no GitHub
# Repositório: leon337/gravador_tela_linux
# ============================================================

REPO_URL="https://github.com/leon337/gravador_tela_linux.git"
BRANCH="main"
COMMIT_MESSAGE="${1:-feat: publicar MVP inicial do gravador de tela Linux}"

log() {
  printf '\n\033[1;36m==> %s\033[0m\n' "$1"
}

fail() {
  printf '\n\033[1;31mERRO: %s\033[0m\n' "$1" >&2
  exit 1
}

command -v git >/dev/null 2>&1 || fail "Git não está instalado."

# Confirma que o script está sendo executado na raiz do projeto.
[[ -f "Cargo.toml" ]] || fail "Cargo.toml não encontrado. Execute este script dentro da pasta gravador-linux-mvp."

log "Preparando o .gitignore"

touch .gitignore

add_ignore() {
  local entry="$1"
  grep -qxF "$entry" .gitignore || printf '%s\n' "$entry" >> .gitignore
}

add_ignore "target/"
add_ignore ".env"
add_ignore ".env.*"
add_ignore "*.log"
add_ignore ".vscode/"
add_ignore ".idea/"
add_ignore "*.swp"
add_ignore "*~"

log "Inicializando o repositório Git"

if [[ ! -d ".git" ]]; then
  git init
fi

git branch -M "$BRANCH"

log "Configurando o repositório remoto"

if git remote get-url origin >/dev/null 2>&1; then
  CURRENT_REMOTE="$(git remote get-url origin)"
  if [[ "$CURRENT_REMOTE" != "$REPO_URL" ]]; then
    printf 'Remote atual: %s\n' "$CURRENT_REMOTE"
    printf 'Novo remote:  %s\n' "$REPO_URL"
    git remote set-url origin "$REPO_URL"
  fi
else
  git remote add origin "$REPO_URL"
fi

log "Adicionando arquivos"

git add .

if git diff --cached --quiet; then
  printf 'Nenhuma alteração nova para criar commit.\n'
else
  git commit -m "$COMMIT_MESSAGE"
fi

log "Verificando se o repositório remoto já possui histórico"

if git ls-remote --exit-code --heads origin "$BRANCH" >/dev/null 2>&1; then
  printf 'A branch remota "%s" já existe.\n' "$BRANCH"
  printf 'Sincronizando o histórico antes do envio...\n'

  # Evita sobrescrever o conteúdo remoto.
  git pull --rebase origin "$BRANCH" || {
    printf '\nNão foi possível concluir o rebase automaticamente.\n'
    printf 'Resolva os conflitos, execute "git rebase --continue" e depois "git push -u origin %s".\n' "$BRANCH"
    exit 1
  }
fi

log "Enviando o projeto para o GitHub"

git push -u origin "$BRANCH"

log "Publicação concluída"
printf 'Repositório: %s\n' "$REPO_URL"
