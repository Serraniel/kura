# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
# Status: machine-assisted, review wanted.

## Command line

cli-usage =
    使い方: kura <コマンド>

    コマンド:
      check      アップデートを確認し、変更内容を表示します
      doctor     問題報告用にこのシステムの概要を表示します
      orphans    どこからも必要とされていないパッケージを表示します
      sbom       インストール済みソフトウェアの一覧を書き出します (CycloneDX)
      version    kura のバージョンを表示します
cli-unknown-command = 不明なコマンドです: { $command }
cli-missing-command = コマンドが指定されていません。

## check

check-running = アップデートを確認しています…
check-none = すべて最新の状態です。
check-found = 利用可能なアップデートが { $count } 件あります。
check-group-apps = アプリ ({ $count })
check-group-system = システムの構成要素 ({ $count })
check-group-replaced = アップデートで置き換えられるもの ({ $count })
check-group-cleanup = 不要になったもの ({ $count })
check-sizes = ダウンロード: { $download } · ディスク使用量: { $delta }
check-preview-only = 何も変更されていません。アップデートのインストールは、今後の kura のバージョンで対応します。
check-blocked = アップデートを準備できません:
problem-unsatisfied = { $target } には { $dependency } が必要ですが、もう提供されていません。
problem-conflict = { $first } と { $second } は同時にインストールできません。
problem-arch = 別の種類のプロセッサー向けにビルドされています: { $packages }
error-check = アップデートを確認できませんでした: { $reason }

## doctor

doctor-title = システムの概要
doctor-distro = ディストリビューション
doctor-packages = インストール済みパッケージ
doctor-explicit = ユーザーがインストール
doctor-foreign = AUR またはローカルでビルド
doctor-orphans = 不要になったもの
doctor-language = 言語

## orphans

orphans-none = 片付けるものはありません。インストール済みのパッケージはすべて使われています。
orphans-found = 不要になったパッケージが { $count } 個あります:
orphans-hint = 次回のアップデート後に、kura がこれらの削除を提案します。

## sbom

sbom-written = ソフトウェア一覧を { $path } に保存しました。

## Errors

error-database = パッケージデータベースを読み込めませんでした: { $reason }
error-write = { $path } に書き込めませんでした: { $reason }

## Window

nav-overview = 概要
nav-history = 履歴
nav-settings = 設定
overview-subtitle = このコンピューターの概要です。
overview-refresh = 再読み込み
prototype-note = これは初期の試作版です。情報を読み取るだけで、何も変更しません。
