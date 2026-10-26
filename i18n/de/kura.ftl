# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later

## Command line

cli-usage =
    Aufruf: kura <Befehl>

    Befehle:
      check      Nach Updates suchen und zeigen, was sich ändern würde
      doctor     Übersicht über dieses System für Problemberichte anzeigen
      history    Gespeicherte Systemstände anzeigen
      mirrors    Schnelle Download-Server auswählen
      orphans    Pakete auflisten, die nichts mehr braucht
      profiles   Zusätzliche Paketquellen hinzufügen oder entfernen: kura profiles [add|remove <Name>]
      sbom       Liste aller installierten Software schreiben (CycloneDX)
      search     Apps in den Paketquellen und im AUR suchen: kura search <Wort>
      update     Alle Updates installieren (zeigt vorher eine Vorschau)
      install    Eine App installieren: kura install <Name>
      kernels    Kernel anzeigen, hinzufügen oder entfernen: kura kernels [add|remove <Name>]
      remove     Eine App entfernen und was nur sie brauchte: kura remove <Name>
      report     Problembericht anzeigen und auf Wunsch auf GitHub öffnen: kura report [was passiert ist]
      security   Bekannte Sicherheitsprobleme installierter Software anzeigen
      settings   Einstellungen anzeigen oder ändern: kura settings aur on|off, kura settings aur-wait <Tage>
      snapshots  Snapshot-Status anzeigen; kura snapshots setup ergänzt, was fehlt
      undo       Zu einem früheren Stand zurückkehren: kura undo [Nummer]
      version    Version von kura anzeigen
cli-unknown-command = Unbekannter Befehl: { $command }
cli-missing-command = Kein Befehl angegeben.

## check

check-running = Suche nach Updates …
check-none = Alles ist aktuell.
check-found =
    { $count ->
        [one] Ein Update ist verfügbar.
       *[other] { $count } Updates sind verfügbar.
    }
check-group-apps = Apps ({ $count })
check-group-system = Systemteile ({ $count })
check-group-replaced = Werden beim Update ersetzt ({ $count })
check-group-cleanup = Nicht mehr benötigt ({ $count })
check-sizes = Download: { $download } · Speicherplatz: { $delta }
check-preview-only = Es wurde nichts verändert. Das Installieren von Updates kommt mit einer späteren Version von kura.
check-blocked = Das Update kann nicht vorbereitet werden:
change-blocked = Die Änderung kann nicht vorbereitet werden:
problem-unsatisfied = { $target } benötigt { $dependency }, das nicht mehr verfügbar ist.
problem-conflict = { $first } und { $second } können nicht gemeinsam installiert werden.
problem-arch = Für einen anderen Prozessortyp gebaut: { $packages }
problem-arch-unnamed = Einige Pakete sind für einen anderen Prozessortyp gebaut.
problem-not-found = Es wurde kein Paket namens { $name } gefunden.
problem-not-restorable = kura kann { $name } { $version } nicht zurückholen: Es stammt aus dem AUR oder wurde auf diesem Computer gebaut, und die Datei ist nicht mehr im Paket-Cache.
undo-title = Zurück zum Stand #{ $id } von { $when }:
undo-limits = Einstellungen und persönliche Dateien werden nicht verändert. Das nächste Update bringt die neueren Versionen zurück. Nutzen Sie das also, um ein Problem zu umgehen, bis eine Korrektur erscheint.
undo-nothing = Das ist bereits der aktuelle Stand.
undo-unknown = Es gibt keinen gespeicherten Stand #{ $id }. kura history listet die gespeicherten Stände auf.
confirm-undo = Zu diesem Stand zurückkehren? [j/N]
mirrors-running = Download-Server in Ihrer Nähe werden getestet …
mirrors-done = Jetzt werden { $count } schnelle Download-Server verwendet. Die bisherige Liste wurde als { $backup } aufbewahrt.
solver-found =
    { $count ->
        [one] kura hat eine Lösung gefunden: Ein alter Teil, den keine App mehr verwendet, blockiert das Update. Er wird entfernt, alles andere wird aktualisiert.
       *[other] kura hat eine Lösung gefunden: { $count } alte Teile, die keine App mehr verwendet, blockieren das Update. Sie werden entfernt, alles andere wird aktualisiert.
    }
solver-tested = kura hat diese Lösung getestet, ohne etwas zu verändern.
error-check = Es konnte nicht nach Updates gesucht werden: { $reason }

## history

history-title = Gespeicherte Systemstände
history-empty = Es wurde noch nichts gespeichert. kura speichert ab jetzt jede Änderung.
history-today = heute
history-yesterday = gestern
history-days-ago =
    { $days ->
        [one] vor einem Tag
       *[other] vor { $days } Tagen
    }
history-installed = { $count } installiert
history-upgraded = { $count } aktualisiert
history-downgraded = { $count } auf ältere Version zurückgesetzt
history-removed = { $count } entfernt
history-start = Ausgangsstand: { $count } Pakete
history-outside = (außerhalb von kura geändert)
history-auto-security = (Sicherheitskorrekturen, automatisch installiert)
list-separator = {", "}

## Applying changes

preview-title = Das wird geändert:
confirm-update = Diese Updates installieren? [j/N]
news-title = Arch-Linux-Neuigkeiten zu diesem Update:
news-unavailable = Die Arch-Linux-Neuigkeiten konnten nicht geladen werden. Bitte lesen Sie vor dem Update https://archlinux.org/news/.
confirm-update-news = Haben Sie die Neuigkeiten oben gelesen und möchten Sie die Updates installieren? [j/N]
confirm-install = Das installieren? [j/N]
confirm-remove = Das entfernen? [j/N]
confirm-yes = j
apply-cancelled = Es wurde nichts verändert.
apply-refreshing = Paketlisten werden aktualisiert …
apply-downloading = { $file } wird heruntergeladen
apply-building = { $name } wird gebaut ({ $current }/{ $total })
apply-checking = Pakete werden geprüft …
apply-installing = { $package } wird installiert ({ $current }/{ $total })
apply-removing = { $package } wird entfernt ({ $current }/{ $total })
apply-hook = Abschluss: { $name }
apply-keyring = Zuerst werden die Signaturschlüssel aktualisiert …
apply-cleanup = Nicht mehr benötigte Teile werden entfernt …
apply-done = Fertig. Die Änderung ist im Verlauf gespeichert.
apply-changed = Seit der Vorschau hat sich etwas geändert. Bitte prüfen Sie die neue Vorschau.
apply-not-authorized = Die Änderung wurde nicht erlaubt. Entweder wurde das Passwort nicht eingegeben, oder dieses Konto darf keine Software installieren.
apply-locked = Ein anderes Programm installiert gerade Software. Bitte versuchen Sie es erneut, wenn es fertig ist.
apply-busy = kura nimmt bereits eine andere Änderung vor. Bitte warten Sie, bis sie abgeschlossen ist.
apply-failed = Die Änderung ist fehlgeschlagen: { $reason }
error-helper = Der Systemdienst von kura ist nicht verfügbar: { $reason }

## Tray

tray-uptodate = Alles ist aktuell
tray-updates =
    { $count ->
        [one] Ein Update ist bereit
       *[other] { $count } Updates sind bereit
    }
tray-attention = Ein Update braucht Ihre Aufmerksamkeit
tray-unknown = kura hat noch nicht nach Updates gesucht
tray-open-hint = Klicken, um kura zu öffnen.
notify-updates-body = Öffnen Sie kura, um vor der Installation zu sehen, was sich ändert.
notify-attention-body = Öffnen Sie kura, um zu sehen, was zu tun ist. Es wurde nichts verändert.
notify-open = kura öffnen
notify-auto-security-title = Sicherheitsupdates wurden installiert
notify-auto-security-body = { $count ->
    [one] Sie beheben ein bekanntes Sicherheitsproblem.
   *[other] Sie beheben { $count } bekannte Sicherheitsprobleme.
} Jede Änderung steht im Verlauf von kura, und Sie können dort zurückkehren.

## Security

security-one-app = { $app } hat ein bekanntes Sicherheitsproblem
security-more-apps =
    { $count ->
        [one] { $app } und eine weitere App haben ein bekanntes Sicherheitsproblem
       *[other] { $app } und { $count } weitere Apps haben ein bekanntes Sicherheitsproblem
    }
security-system = Das System hat ein bekanntes Sicherheitsproblem
security-fix = Ein Update behebt es. Bitte installieren Sie es bald.
security-nofix = Es gibt noch keine Korrektur. kura prüft jeden Tag und meldet sich, sobald es eine gibt.
security-none = Keine bekannten Sicherheitsprobleme.
security-running = Bekannte Sicherheitsprobleme werden geprüft …
security-unavailable = Die Liste bekannter Sicherheitsprobleme konnte nicht geladen werden.
security-detail = { $package } { $installed }: { $kind }
security-fixed-in = Behoben in { $version }
security-used-by = Verwendet von: { $apps }
severity-low = geringes Risiko
severity-medium = mittleres Risiko
severity-high = hohes Risiko
severity-critical = kritisches Risiko

## doctor

doctor-title = Systemübersicht
doctor-distro = Distribution
doctor-packages = Installierte Pakete
doctor-explicit = Bewusst installiert
doctor-foreign = Aus dem AUR oder selbst gebaut
doctor-orphans = Nicht mehr benötigt
doctor-language = Sprache

## orphans

orphans-none = Nichts aufzuräumen. Alle installierten Pakete werden noch gebraucht.
orphans-found =
    { $count ->
        [one] Ein Paket wird nicht mehr gebraucht:
       *[other] { $count } Pakete werden nicht mehr gebraucht:
    }
orphans-hint = kura bietet nach dem nächsten Update an, sie zu entfernen.

## sbom

sbom-written = Softwareliste gespeichert unter { $path }.

## Errors

error-database = Die Paketdatenbank konnte nicht gelesen werden: { $reason }
error-write = { $path } konnte nicht geschrieben werden: { $reason }

## Window

nav-overview = Übersicht
nav-updates = Updates
gui-review = Ansehen und installieren
gui-check-now = Jetzt nach Updates suchen
gui-install = Installieren
gui-install-news = Ich habe die Neuigkeiten gelesen. Installieren
gui-read-news = Ankündigung lesen
gui-go-back = Zu diesem Stand zurückkehren
gui-cancel = Abbrechen
gui-updates-intro = Das ist alles, was sich ändert. Es passiert nichts, bevor Sie auf Installieren klicken.
gui-tray-mono = Einfarbiges Symbol im Systemabschnitt
gui-tray-mono-hint = Passt zu den anderen Symbolen in der Kontrollleiste.
gui-aur = Software aus dem AUR
gui-aur-hint = Das AUR enthält Software, die andere Nutzer teilen. Arch Linux prüft sie nicht. kura baut sie auf diesem Computer in einer getrennten, eingeschränkten Umgebung.
gui-aur-wait-hint = Neue AUR-Versionen warten so lange, bevor kura sie installiert, damit Probleme vorher auffallen. Wenn Sie eine App von Hand installieren, nimmt kura immer die neueste Version.
gui-profiles = Zusätzliche Paketquellen
gui-add = Hinzufügen
gui-profile-add = { $name } hinzufügen
gui-profile-remove = { $name } entfernen
gui-switch = Jetzt umstellen
gui-aur-after = { $count ->
    [one] Danach ist ein Update aus dem AUR bereit. Prüfen Sie erneut, um es zu bauen.
   *[other] Danach sind { $count } Updates aus dem AUR bereit. Prüfen Sie erneut, um sie zu bauen.
}
gui-last-check = Zuletzt geprüft: { $when }.
nav-health = Systemzustand
nav-software = Software
health-cpu = Prozessor
health-gpu = Grafik
health-memory = Arbeitsspeicher
health-disk = Speicher
health-disk-used = { $used } von { $total } belegt
health-cache = Alte Update-Dateien: { $size }
health-kernels = Kernel
health-kernel-running = in Verwendung
kernels-title = Kernel: der Kern des Systems. Mehrere können installiert sein; beim Start wählen Sie einen aus.
kernels-usage = Aufruf: kura kernels [add|remove <Name>]
kernel-keep-running = { $name } läuft gerade. Starten Sie den Computer zuerst mit einem anderen Kernel.
kernel-keep-last = { $name } ist der einzige Kernel. Ohne ihn könnte der Computer nicht starten.
kernel-linux = Der neueste stabile Kernel von Arch Linux.
kernel-linux-lts = Langzeitunterstützung: ändert sich selten. Ein guter zweiter Kernel, falls ein neuer ein Problem hat.
kernel-linux-zen = Für Desktop und Spiele abgestimmt, reagiert etwas schneller.
kernel-linux-hardened = Zusätzlicher Schutz vor Angriffen. Manche Apps und Spiele laufen damit nicht.
kernel-linux-cachyos = Der Kernel von CachyOS, auf Geschwindigkeit abgestimmt.
kernel-linux-cachyos-lts = Der Langzeit-Kernel von CachyOS.
health-suggestions = Vorschläge
health-snapshots = Snapshots
snapshots-on = Vor jedem Update durch { $tool }
snapshots-off = Nicht eingerichtet
snapshots-partial = Teilweise eingerichtet
snapshots-unsupported = Mit diesem Dateisystem nicht möglich
snapshots-missing = Es fehlt:
snapshots-step-install = { $package } installieren
snapshots-step-config = snapper-Konfiguration für / anlegen
snapshots-step-enable = { $unit } einschalten
action-trust-key = Dem Signaturschlüssel { $key } vertrauen
action-forget-key = Dem Signaturschlüssel { $key } nicht mehr vertrauen
action-install-from = { $files } herunterladen und installieren
action-add-repo = Paketquelle { $name } hinzufügen
action-remove-repo = Paketquelle { $name } entfernen
action-bootstrap-repo = Paketquelle { $name } mit ihrem Hauptserver hinzufügen, um Schlüsselbund und Serverlisten zu laden
action-arch = Pakete verwenden, die für { $arch } gebaut sind
action-arch-generic = Wieder die Pakete für alle x86-64-Prozessoren verwenden
action-regenerate-grub = Startmenü aktualisieren
profiles-title = Zusätzliche Paketquellen, die kura hinzufügen und wieder entfernen kann:
profile-on = hinzugefügt
profile-off = nicht hinzugefügt
profiles-usage = Aufruf: kura profiles [add|remove|switch <Name>]
profile-already = { $name } ist bereits hinzugefügt.
profile-missing = { $name } ist nicht hinzugefügt.
profile-chaotic-aur-text = Fertig gebaute Pakete für viele Apps aus dem AUR, sodass sie in Sekunden installiert sind, statt auf diesem Computer gebaut zu werden. Sie wird von Freiwilligen betrieben, nicht von Arch Linux: Wer sie hinzufügt, vertraut ihnen sein System an.
confirm-profile-add = Diese Paketquelle hinzufügen? [j/N]
confirm-profile-remove = Diese Paketquelle entfernen? Apps, die Sie daraus installiert haben, bleiben. [j/N]
profile-cpu = Dieser Prozessor ist zu alt für { $name }.
profile-switch-estimate = Wenn jetzt alles umgestellt wird, ändern sich { $count } Pakete und { $download } werden heruntergeladen. Sonst wechseln die Pakete mit ihren nächsten Updates.
profile-switch-back-estimate = Das Zurückwechseln ändert { $count } Pakete und lädt { $download } herunter.
confirm-profile-switch = Alle installierten Pakete jetzt umstellen? (Nein: Sie wechseln mit ihren nächsten Updates) [j/N]
profile-switch-title = Installierte Pakete umstellen:
confirm-switch = Diese Pakete jetzt umstellen? [j/N]
profile-cachyos-text = Pakete, die für neuere Prozessoren gebaut sind (x86-64-v3 oder v4). Anspruchsvolle Apps können dadurch einige Prozent schneller werden. Betrieben vom CachyOS-Team, nicht von Arch Linux: Wer sie hinzufügt, vertraut ihnen sein System an. kura kann sie wieder entfernen und jedes Paket zurückstellen.
snapshots-setup-hint = Um zu ergänzen, was fehlt: kura snapshots setup
snapshots-nothing = Es gibt nichts einzurichten.
preview-actions = Diese Einstellungen ändern sich:
confirm-snapshots = Snapshots einrichten? [j/N]
suggest-backup-kernel = Einen Ersatz-Kernel installieren
suggest-backup-kernel-text = Falls ein Update den Computer einmal nicht mehr starten lässt, kann der Ersatz im Startmenü ausgewählt werden.
suggest-snapshots = Automatische Snapshots einschalten
suggest-snapshots-text = Ein Snapshot sichert den Zustand des Systems vor jedem Update, damit Sie zurückkehren können, wenn etwas kaputtgeht. kura behält, was schon eingerichtet ist, und ergänzt nur, was fehlt.
suggest-mirrors = Schnellere Download-Server auswählen
suggest-mirrors-text = kura testet Server in Ihrer Nähe und verwendet die schnellsten.
suggest-cleanup =
    { $count ->
        [one] Ein Paket wird nicht mehr gebraucht
       *[other] { $count } Pakete werden nicht mehr gebraucht
    }
suggest-cleanup-text = Sie werden mit dem nächsten Update entfernt.
driver-microcode = Prozessor-Korrekturen von { $vendor }
driver-microcode-text = { $vendor } veröffentlicht Korrekturen für Fehler in Prozessoren, auch für Sicherheitsprobleme. Sie werden bei jedem Start geladen. Jeder Computer mit einem Prozessor von { $vendor } sollte sie haben.
driver-graphics = Grafiktreiber für { $vendor }
driver-graphics-text = Damit Spiele, Videos und der Desktop Ihre Grafikkarte voll nutzen können.
doctor-drivers = Für diese Hardware empfohlen:
search-placeholder = Apps suchen, zum Beispiel „Foto“ oder „Musik“
search-none = Keine Apps gefunden. Versuchen Sie ein anderes Wort.
search-repos = Aus den Paketquellen:
search-aur = Aus dem AUR (von der Community geteilt, wird auf diesem Computer gebaut):
search-aur-failed = Das AUR konnte nicht durchsucht werden: { $reason }
aur-votes = { $count ->
    [one] { $count } Stimme
   *[other] { $count } Stimmen
}
aur-new = neu
aur-orphaned = ohne Betreuer
aur-out-of-date = als veraltet markiert
aur-mixed = Einige davon kommen aus dem AUR. Installieren Sie diese getrennt: kura install { $names }
aur-build-list = Wird auf diesem Computer aus dem AUR gebaut:
aur-notice = AUR-Pakete werden von anderen Nutzern geteilt und nicht von Arch Linux geprüft. kura baut sie in einer getrennten, eingeschränkten Umgebung.
confirm-aur = Das bauen und installieren? [j/N]
aur-off = { $names } ist nicht in den Paketquellen, und das AUR ist ausgeschaltet. Zum Einschalten: kura settings aur on
aur-check-failed = AUR-Updates konnten nicht geprüft werden: { $reason }
check-aur = Aus dem AUR: { $ready } bereit, { $waiting } warten noch
check-rebuild = Diese Apps aus dem AUR brauchen eine Bibliothek, die ein Update ersetzt hat. kura update baut sie neu:
aur-updates-title = Updates aus dem AUR:
aur-waiting = { $count ->
    [one] Ein AUR-Update wartet
   *[other] { $count } AUR-Updates warten
}, bis { $count ->
    [one] es
   *[other] sie
} { $days } Tage alt { $count ->
    [one] ist
   *[other] sind
}, damit Probleme in neuen Versionen vorher auffallen: { $names }
confirm-aur-update = Diese Updates bauen und installieren? [j/N]
settings-aur = AUR
settings-aur-wait = Wartezeit vor AUR-Updates
settings-on = an
settings-off = aus
settings-days = { $days ->
    [one] { $days } Tag
   *[other] { $days } Tage
}
settings-saved = Gespeichert.
settings-auto-security = Sicherheitskorrekturen automatisch installieren
gui-auto-security-hint = Wenn ein Update ein bekanntes Sicherheitsproblem behebt, installiert kura es ohne Rückfrage, aber nur, wenn der Probelauf kein Problem gefunden hat und keine Arch-Nachricht es betrifft. Danach erhalten Sie eine Meldung und können im Verlauf zurückkehren.
settings-usage = Aufruf: kura settings [aur on|off] [aur-wait <0–14>] [auto-security on|off]
report-intro = Dieser Bericht enthält keinen Benutzernamen, Computernamen und keinen persönlichen Ordner. Bitte lesen Sie ihn, bevor Sie ihn teilen:
confirm-report = Als vorausgefüllten Problembericht auf GitHub im Browser öffnen? [j/N]
report-opened = Falls sich kein Browser geöffnet hat, kopieren Sie diese Adresse: { $url }
gui-report = Dieses Problem melden
gui-report-open = Auf GitHub öffnen
gui-installed = Installiert
gui-remove = Entfernen
gui-install-title = { $name } installieren
gui-remove-title = { $name } entfernen
gui-setup = Einrichten
gui-details = Details
nav-history = Verlauf
nav-settings = Einstellungen
overview-subtitle = Ein kurzer Überblick über diesen Computer.
overview-refresh = Aktualisieren
