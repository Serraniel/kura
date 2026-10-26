# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
# Status: machine-assisted, review wanted.

## Command line

cli-usage =
    使い方: kura <コマンド>

    コマンド:
      bundles    アプリと設定をまとめたセット: kura bundles [add <名前>]
      check      アップデートを確認し、変更内容を表示します
      doctor     問題報告用にこのシステムの概要を表示します
      history    保存されたシステムの状態を表示します
      mirrors    速いダウンロードサーバーを選びます
      orphans    どこからも必要とされていないパッケージを表示します
      profiles   追加のリポジトリを追加・削除します: kura profiles [add|remove <名前>]
      sbom       インストール済みソフトウェアの一覧を書き出します (CycloneDX)
      search     リポジトリと AUR でアプリを検索します: kura search <単語>
      update     すべてのアップデートをインストールします (先にプレビューを表示します)
      install    アプリをインストールします: kura install <名前>
      info       パッケージの詳細 (AUR は信頼の目安付き): kura info <名前>
      kernels    カーネルを表示・追加・削除します: kura kernels [add|remove <名前>]
      remove     アプリと、そのアプリだけが使っていたものを削除します: kura remove <名前>
      report     問題レポートを表示し、必要なら GitHub で開きます: kura report [起きたこと]
      security   インストール済みソフトウェアの既知のセキュリティ問題を表示します
      settings   設定を表示・変更します: kura settings aur on|off、kura settings aur-wait <日数>
      snapshots  スナップショットの状態を表示します。kura snapshots setup で不足分を追加します
      undo       以前の状態に戻します: kura undo [番号]
      version    kura のバージョンを表示します
cli-unknown-command = 不明なコマンドです: { $command }
pacman-partial = 後のインストールで古いパッケージと新しいパッケージが混ざるため、kura はアップデートせずにパッケージ一覧だけを更新することはしません。kura -Syu または kura update を使ってください。
pacman-clean = kura は以前の状態に戻せるよう、古いパッケージファイルを保持しています。システムの状態ページに表示されます。
pacman-usage = kura は -Syu、-S <名前>、-R <名前>、-Ss <単語>、-Si <名前>、-Q、-Qi <名前>、-Qs <単語> を理解します。その他は kura --help を参照してください。
cli-missing-command = コマンドが指定されていません。

## check

check-running = アップデートを確認しています…
check-none = すべて最新の状態です。
check-found = 利用可能なアップデートが { $count } 件あります。
check-group-apps = アプリ ({ $count })
check-group-system = システムの構成要素 ({ $count })
check-group-replaced = アップデートで置き換えられるもの ({ $count })
check-group-reinstall = アップデートのために削除し、直後に再インストールするもの ({ $count })
check-group-build-tools = これらのアプリのビルドにだけ必要で、後で削除するもの ({ $count })
detail-rebuild = 再ビルド ({ $version })
aur-rebuild-why = これらの AUR アプリは、アップデートで置き換えられたライブラリを必要とするため再ビルドします: { $list }
reinstall-now = 修復で削除したものを再インストールしています:
check-group-cleanup = 不要になったもの ({ $count })
check-sizes = ダウンロード: { $download } · ディスク使用量: { $delta }
check-preview-only = 何も変更されていません。アップデートのインストールは、今後の kura のバージョンで対応します。
check-blocked = アップデートを準備できません:
change-blocked = 変更を準備できません:
problem-unsatisfied = { $target } には { $dependency } が必要ですが、もう提供されていません。
problem-conflict = { $first } と { $second } は同時にインストールできません。
problem-arch = 別の種類のプロセッサー向けにビルドされています: { $packages }
problem-arch-unnamed = 一部のパッケージは別の種類のプロセッサー向けにビルドされています。
problem-not-found = { $name } という名前のパッケージは見つかりませんでした。
problem-not-restorable = { $name } { $version } を元に戻せません。AUR から入手したか、このコンピューターでビルドしたもので、パッケージキャッシュにファイルが残っていません。
undo-title = { $when } の状態 #{ $id } に戻します:
undo-limits = 設定や個人のファイルは変更されません。次のアップデートで新しいバージョンに戻るため、修正が出るまで問題を回避するために使ってください。
undo-nothing = これはすでに現在の状態です。
undo-unknown = 保存された状態 #{ $id } はありません。kura history で保存された状態を確認できます。
confirm-undo = この状態に戻しますか? [y/N]
mirrors-running = 近くのダウンロードサーバーをテストしています…
mirrors-done = 速いダウンロードサーバー { $count } 台を使うようになりました。以前の一覧は { $backup } として保存されています。
solver-found = kura が解決方法を見つけました: どのアプリも使っていない古い構成要素 { $count } 個がアップデートを妨げています。これらを削除し、ほかはすべて更新します。
solver-tested = kura は何も変更せずにこの解決方法をテストしました。
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
history-auto-security = (セキュリティ修正、自動インストール)
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
progress-of = { $done } / { $total }
progress-under-minute = 残り 1 分未満
progress-minutes = 残り約 { $minutes } 分
apply-trial = まずシステムのコピーでアップデートを試しています…
apply-trial-failed = システムのコピーでアップデートが失敗したため、kura は何も変更していません。理由: { $reason }
apply-downloading = { $file } をダウンロードしています
apply-building = { $name } をビルドしています ({ $current }/{ $total })
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
notify-auto-security-title = セキュリティアップデートをインストールしました
notify-auto-security-body = 既知のセキュリティ問題 { $count } 件を修正します。すべての変更は kura の履歴で確認でき、そこから元に戻せます。

## Security

security-one-app = { $app } に既知のセキュリティ問題があります
security-more-apps = { $app } とほかのアプリ { $count } 件に既知のセキュリティ問題があります
security-system = システムに既知のセキュリティ問題があります
security-fix = アップデートで修正されます。早めにインストールしてください。
security-nofix = まだ修正はありません。kura が毎日確認し、修正が出たらお知らせします。
security-none = 既知のセキュリティ問題はありません。
security-running = 既知のセキュリティ問題を確認しています…
security-unavailable = 既知のセキュリティ問題の一覧を読み込めませんでした。
security-detail = { $package } { $installed }: { $kind }
security-fixed-in = 修正済みバージョン: { $version }
security-used-by = 使用しているアプリ: { $apps }
severity-low = 低リスク
severity-medium = 中リスク
severity-high = 高リスク
severity-critical = 重大なリスク

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
nav-updates = アップデート
gui-review = 確認してインストール
gui-check-now = 今すぐアップデートを確認
gui-install = インストール
gui-install-news = お知らせを読みました。インストール
gui-read-news = お知らせを読む
gui-go-back = この状態に戻す
gui-cancel = キャンセル
gui-updates-intro = 変更されるのは以下のすべてです。「インストール」を押すまで何も起こりません。
gui-tray-mono = トレイアイコンを単色にする
gui-tray-mono-hint = パネルのほかのアイコンと合わせます。
gui-aur = AUR のソフトウェア
gui-aur-hint = AUR には他のユーザーが共有するソフトウェアがあります。Arch Linux による確認は受けていません。kura はこのコンピューター上の分離された制限付きの環境でビルドします。
gui-aur-wait-hint = 問題が先に見つかるよう、AUR の新しいバージョンはこの期間待ってからインストールされます。アプリを手動でインストールすると、常に最新バージョンになります。
gui-profiles = 追加のリポジトリ
gui-bundles = すぐ使えるセット
gui-add = 追加
gui-profile-add = { $name } を追加
gui-profile-remove = { $name } を削除
gui-switch = 今すぐ切り替え
gui-aur-after = この後、AUR のアップデート { $count } 件が準備できています。もう一度確認するとビルドできます。
gui-last-check = 最終確認: { $when }。
nav-health = システムの状態
nav-software = ソフトウェア
health-cpu = プロセッサー
health-gpu = グラフィックス
health-memory = メモリー
health-disk = ストレージ
health-disk-used = { $total } 中 { $used } を使用
health-cache = 古いアップデートファイル: { $size }
health-kernels = カーネル
health-kernel-running = 使用中
kernels-title = カーネル: システムの中核です。複数インストールでき、起動時に選べます。
kernels-usage = 使い方: kura kernels [add|remove <名前>]
bundles-title = すぐ使えるセット: アプリと必要な設定をまとめたものです。
bundles-usage = 使い方: kura bundles [add <名前> [追加…]]
kernel-keep-running = { $name } は現在使用中です。先に別のカーネルでコンピューターを起動してください。
kernel-keep-last = { $name } は唯一のカーネルです。これがないとコンピューターは起動できません。
kernel-old = { $new } は問題なく起動しました。古いカーネル { $old } は削除できます: kura kernels remove { $old }
suggest-old-kernel = 古いカーネル { $old } を削除
kernel-linux = Arch Linux の最新の安定版カーネル。
kernel-linux-lts = 長期サポート版。変更が少なく、新しいカーネルに問題があるときの予備に最適です。
kernel-linux-zen = デスクトップやゲーム向けに調整され、反応が少し速くなります。
kernel-linux-hardened = 攻撃への保護を強化。一部のアプリやゲームは動作しない場合があります。
kernel-linux-cachyos = 速度重視で調整された CachyOS のカーネル。
kernel-linux-cachyos-lts = CachyOS の長期サポート版カーネル。
health-suggestions = 提案
health-snapshots = スナップショット
snapshots-on = アップデートの前に毎回 { $tool } が作成
snapshots-off = 未設定
snapshots-partial = 一部のみ設定済み
snapshots-unsupported = このファイルシステムでは利用できません
snapshots-missing = 不足しているもの:
snapshots-step-install = { $package } をインストール
snapshots-step-config = / の snapper 設定を作成
snapshots-step-enable = { $unit } を有効化
action-trust-key = 署名鍵 { $key } を信頼する
action-forget-key = 署名鍵 { $key } の信頼を取り消す
action-install-from = { $files } をダウンロードしてインストール
action-add-repo = リポジトリ { $name } を追加
action-remove-repo = リポジトリ { $name } を削除
action-bootstrap-repo = 鍵束とミラーリストを取得するため、リポジトリ { $name } をメインサーバーで追加
action-arch = { $arch } 向けにビルドされたパッケージを使用
action-arch-generic = すべての x86-64 プロセッサー向けのパッケージに戻す
action-regenerate-grub = 起動メニューを更新
action-default-kernel = 既定で { $name } で起動する
action-user-group = { $user } をグループ { $group } に追加
profiles-title = kura が追加・削除できる追加のリポジトリ:
profile-on = 追加済み
profile-off = 未追加
profiles-usage = 使い方: kura profiles [add|remove|switch <名前>]
profile-already = { $name } はすでに追加されています。
profile-missing = { $name } は追加されていません。
profile-chaotic-aur-text = AUR の多くのアプリのビルド済みパッケージです。このコンピューターでビルドせずに数秒でインストールできます。運営は Arch Linux ではなくボランティアです。追加すると、システムを彼らに任せることになります。
confirm-profile-add = このリポジトリを追加しますか? [y/N]
confirm-profile-remove = このリポジトリを削除しますか? そこからインストールしたアプリは残ります。[y/N]
profile-cpu = このプロセッサーは { $name } には古すぎます。
profile-switch-estimate = 今すべて切り替えると { $count } 個のパッケージが変更され、{ $download } をダウンロードします。切り替えない場合は、次のアップデートで順次切り替わります。
profile-switch-back-estimate = 元に戻すと { $count } 個のパッケージが変更され、{ $download } をダウンロードします。
confirm-profile-switch = インストール済みのパッケージをすべて今切り替えますか? (いいえ: 次のアップデートで切り替わります) [y/N]
profile-switch-title = インストール済みパッケージの切り替え:
confirm-switch = これらのパッケージを今切り替えますか? [y/N]
profile-cachyos-text = 新しいプロセッサー (x86-64-v3 または v4) 向けにビルドされたパッケージで、負荷の高いアプリが数パーセント速くなることがあります。運営は Arch Linux ではなく CachyOS チームです。追加すると、システムを彼らに任せることになります。kura はいつでも削除して、すべてのパッケージを元に戻せます。
snapshots-setup-hint = 不足分を追加するには: kura snapshots setup
snapshots-nothing = 設定するものはありません。
preview-actions = 変更される設定:
confirm-snapshots = スナップショットを設定しますか? [y/N]
suggest-backup-kernel = 予備のカーネルをインストール
suggest-backup-kernel-text = アップデートの後にコンピューターが起動しなくなった場合でも、起動メニューで予備のカーネルを選べます。
suggest-snapshots = 自動スナップショットを有効にする
suggest-snapshots-text = スナップショットは、アップデートの前に毎回システムの状態を保存します。問題が起きたときに元に戻せます。kura は既存の設定をそのまま残し、不足しているものだけを追加します。
suggest-mirrors = より速いダウンロードサーバーを選ぶ
suggest-mirrors-text = kura が近くのサーバーをテストし、最も速いものを使います。
suggest-cleanup = 不要になったパッケージが { $count } 個あります
suggest-cleanup-text = 次回のアップデートで削除されます。
driver-microcode = { $vendor } のプロセッサー修正
driver-microcode-text = { $vendor } はセキュリティ問題を含むプロセッサーの不具合の修正を公開しています。起動のたびに読み込まれます。{ $vendor } のプロセッサーを搭載したコンピューターには必ず入れてください。
driver-graphics = { $vendor } のグラフィックドライバー
driver-graphics-text = ゲーム、動画、デスクトップでグラフィックカードの性能を十分に引き出します。
doctor-drivers = このハードウェアにおすすめ:
search-placeholder = アプリを検索 (例:「写真」「音楽」)
search-none = アプリが見つかりませんでした。別の言葉で試してください。
search-repos = リポジトリから:
search-aur = AUR から (コミュニティが共有、このコンピューターでビルド):
search-aur-failed = AUR を検索できませんでした: { $reason }
aur-votes = { $count } 票
aur-new = 新規
aur-orphaned = メンテナーなし
aur-out-of-date = 古いと報告済み
info-source = 提供元
info-version = バージョン
info-description = 説明
info-signals = 票と注意
info-popularity = 人気度
info-maintainer = メンテナー
info-first = 最初の公開
info-changed = 最終更新
info-website = ウェブサイト
info-files = ビルドファイル
aur-mixed = 一部は AUR から提供されています。それらは別にインストールしてください: kura install { $names }
aur-build-list = AUR からこのコンピューターでビルドするもの:
aur-notice = AUR のパッケージは他のユーザーが共有しているもので、Arch Linux による確認は受けていません。kura は分離された制限付きの環境でビルドします。
confirm-aur = これをビルドしてインストールしますか? [y/N]
aur-off = { $names } はリポジトリにありません。また、AUR はオフになっています。オンにするには: kura settings aur on
aur-check-failed = AUR のアップデートを確認できませんでした: { $reason }
check-aur = AUR から: { $ready } 件準備完了、{ $waiting } 件待機中
check-rebuild = これらの AUR アプリは、アップデートで置き換えられたライブラリを必要としています。kura update で再ビルドします:
aur-updates-title = AUR からのアップデート:
aur-waiting = 新しいバージョンの問題が先に見つかるよう、{ $count } 件の AUR アップデートは公開から { $days } 日経つまで待機します: { $names }
confirm-aur-update = これらのアップデートをビルドしてインストールしますか? [y/N]
settings-aur = AUR
settings-aur-wait = AUR アップデートまでの待機期間
settings-on = オン
settings-off = オフ
settings-days = { $days } 日
settings-saved = 保存しました。
settings-auto-security = セキュリティ修正を自動でインストール
gui-auto-security-hint = アップデートが既知のセキュリティ問題を修正する場合、試験実行で問題がなく、関係する Arch のお知らせもなければ、kura は確認なしでインストールします。その後お知らせが表示され、履歴から元に戻せます。
settings-trial = アップデートをまずコピーで試す
gui-trial-hint = btrfs では、kura は何かを変更する前に、使い捨てのシステムのコピーで各アップデートを試します。時間はかかりますが、失敗するアップデートがシステムに届くことはありません。
settings-usage = 使い方: kura settings [aur on|off] [aur-wait <0–14>] [auto-security on|off] [trial on|off]
report-intro = このレポートにはユーザー名、コンピューター名、ホームフォルダーは含まれていません。共有する前に内容を確認してください:
confirm-report = 入力済みの問題レポートとして GitHub をブラウザーで開きますか? [y/N]
report-opened = ブラウザーが開かない場合は、このアドレスをコピーしてください: { $url }
gui-report = この問題を報告
gui-report-open = GitHub で開く
gui-installed = インストール済み
gui-remove = 削除
gui-install-title = { $name } をインストール
gui-remove-title = { $name } を削除
gui-setup = 設定する
gui-details = 詳細
nav-history = 履歴
nav-settings = 設定
overview-subtitle = このコンピューターの概要です。
overview-refresh = 再読み込み
