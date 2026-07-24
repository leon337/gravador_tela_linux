# Gravador Linux — MVP

Aplicação em Rust com GTK4 que usa FFmpeg para gravar tela e áudio no Linux Mint.

## O que esta versão faz

- grava a tela inteira em sessões X11;
- grava microfone;
- grava áudio do sistema;
- mistura microfone e áudio do sistema;
- grava somente áudio;
- exporta WebM/Opus ou MP4/M4A;
- instala automaticamente dependências ausentes;
- compila o projeto e cria um atalho no menu.

## Limitação atual

A captura de tela desta primeira versão funciona em X11. Em uma sessão Wayland, o aplicativo permite gravações somente de áudio, mas informa que a captura da tela exige X11. A próxima etapa técnica é integrar `xdg-desktop-portal` e PipeWire diretamente.

## Instalação com dois cliques

1. Extraia `gravador-linux-mvp.zip`.
2. Clique com o botão direito em `install.desktop`.
3. Marque como confiável ou escolha **Permitir iniciar**.
4. Dê dois cliques em `install.desktop`.
5. Digite sua senha quando o Linux solicitar autorização para instalar pacotes.
6. O instalador pula tudo que já estiver corretamente instalado.
7. Ao final, o aplicativo aparece em **Menu → Som e Vídeo → Gravador Linux**.

Caso o gerenciador de arquivos abra o launcher como texto, use:

```bash
cd gravador-linux-mvp
chmod +x install.sh verify-environment.sh uninstall.sh
./install.sh
```

## Dependências preparadas

Rust, Cargo, GTK4, Libadwaita, GStreamer, PipeWire, FFmpeg, ferramentas de compilação, `pkg-config`, Git, CMake, Meson, Ninja e portais de desktop.

## Uso

1. Escolha o modo.
2. Escolha WebM/Opus para arquivos menores ou MP4/M4A para compatibilidade.
3. Escolha a qualidade.
4. Informe a pasta de saída.
5. Clique em **Iniciar gravação**.
6. Clique em **Parar** para finalizar corretamente o arquivo.

O áudio do sistema é capturado pela fonte monitor da saída padrão, localizada por `pactl get-default-sink`.

## Verificação

```bash
./verify-environment.sh
```

## Desenvolvimento

```bash
cargo run
cargo build --release
```

## Desinstalação

```bash
./uninstall.sh
```

O desinstalador preserva as dependências do sistema para não remover componentes utilizados por outros programas.

## Segurança

O instalador inteiro não roda como root. `sudo` é utilizado somente pelo APT. Rust e Cargo são instalados no usuário atual pelo instalador oficial `rustup`.

## Licença

MIT.
