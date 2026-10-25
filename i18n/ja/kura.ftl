# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
# Status: machine-assisted, review wanted.

## Command line

cli-usage =
    使い方: kura <コマンド>

    コマンド:
      check      アップデートを確認し、変更内容を表示します
      doctor     問題報告用にこのシステムの概要を表示します
      history    保存されたシステムの状態を表示します
      orphans    どこからも必要とされていないパッケージを表示します
      sbom       インストール済みソフトウェアの一覧を書き出します (CycloneDX)
      update     すべてのアップデートをインストールします (先にプレビューを表示します)
      install    アプリをインストールします: kura install <名前>
      remove     アプリと、そのアプリだけが使っていたものを削除します: kura remove <名前>
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
problem-not-found = { $name } という名前のパッケージは見つかりませんでした。
error-check = アップデートを確認できませんでした: { $reason }

## history

history-title = 保存されたシステムの状態
history-empty = まだ何も保存されていません。これから kura は変更のたびに保存します。
history-today = 今日
history-yesterday = 昨日
history-days-ago = { $days } 日前
history-installed = インストール { $count } 件
history-upgraded = 更新 { $count } 件
history-downgraded = 以前のバージョンに戻したもの { $count } 件
history-removed = 削除 { $count } 件
history-start = 起点: { $count } 個のパッケージ
history-outside = (kura 以外で変更)
list-separator = {"、"}

## Applying changes

preview-title = 次のように変更されます:
confirm-update = これらのアップデートをインストールしますか? [y/N]
news-title = このアップデートに関する Arch Linux のお知らせ:
news-unavailable = Arch Linux のお知らせを読み込めませんでした。アップデートの前に https://archlinux.org/news/ をお読みください。
confirm-update-news = 上のお知らせを読んだうえで、アップデートをインストールしますか? [y/N]
confirm-install = これをインストールしますか? [y/N]
confirm-remove = これを削除しますか? [y/N]
confirm-yes = y
apply-cancelled = 何も変更されていません。
apply-refreshing = パッケージ一覧を更新しています…
apply-downloading = { $file } をダウンロードしています
apply-checking = パッケージを確認しています…
apply-installing = { $package } をインストールしています ({ $current }/{ $total })
apply-removing = { $package } を削除しています ({ $current }/{ $total })
apply-hook = 仕上げ: { $name }
apply-keyring = 先に署名鍵を更新しています…
apply-cleanup = 不要になったものを削除しています…
apply-done = 完了しました。変更は履歴に保存されています。
apply-changed = プレビューの後に変更がありました。新しいプレビューを確認してください。
apply-not-authorized = 変更は許可されませんでした。パスワードが入力されなかったか、このアカウントにはソフトウェアをインストールする権限がありません。
apply-locked = 別のプログラムがソフトウェアをインストールしています。終わってからもう一度お試しください。
apply-busy = kura は別の変更を行っています。終わるまでお待ちください。
apply-failed = 変更に失敗しました: { $reason }
error-helper = kura のシステムサービスを利用できません: { $reason }

## Tray

tray-uptodate = すべて最新の状態です
tray-updates = { $count } 件のアップデートの準備ができました
tray-attention = 確認が必要なアップデートがあります
tray-unknown = kura はまだアップデートを確認していません
tray-open-hint = クリックすると kura を開きます。
notify-updates-body = インストールする前に、kura を開いて変更内容を確認してください。
notify-attention-body = kura を開いて対応方法を確認してください。何も変更されていません。
notify-open = kura を開く

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
