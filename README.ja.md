# sixel-playground

## ローカルでドッグフーディングする

リポジトリ上の最新版をローカルにインストールして動作確認する場合は、以下を実行します。

```bash
cargo install --force --git https://github.com/cat2151/sixel-playground wav-viewer
cargo install --force --git https://github.com/cat2151/sixel-playground wav-viewer-ratatui
cargo install --force --git https://github.com/cat2151/sixel-playground ym2151-envelope
cargo install --force --git https://github.com/cat2151/sixel-playground ym2151-envelope-ratatui
```

## アプリ実行コマンド

```bash
wav-viewer <path/to/file.wav>
wav-viewer-ratatui <path/to/file.wav>
ym2151-envelope [AR] [D1R] [D1L] [D2R] [RR]
ym2151-envelope-ratatui [AR] [D1R] [D1L] [D2R] [RR]
```
