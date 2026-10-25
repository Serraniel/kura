# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later

## Command line

cli-usage =
    Aufruf: kura <Befehl>

    Befehle:
      check      Nach Updates suchen und zeigen, was sich ändern würde
      doctor     Übersicht über dieses System für Problemberichte anzeigen
      history    Gespeicherte Systemstände anzeigen
      orphans    Pakete auflisten, die nichts mehr braucht
      sbom       Liste aller installierten Software schreiben (CycloneDX)
      update     Alle Updates installieren (zeigt vorher eine Vorschau)
      install    Eine App installieren: kura install <Name>
      remove     Eine App entfernen und was nur sie brauchte: kura remove <Name>
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
problem-unsatisfied = { $target } benötigt { $dependency }, das nicht mehr verfügbar ist.
problem-conflict = { $first } und { $second } können nicht gemeinsam installiert werden.
problem-arch = Für einen anderen Prozessortyp gebaut: { $packages }
problem-not-found = Es wurde kein Paket namens { $name } gefunden.
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
nav-history = Verlauf
nav-settings = Einstellungen
overview-subtitle = Ein kurzer Überblick über diesen Computer.
overview-refresh = Aktualisieren
prototype-note = Dies ist ein früher Prototyp. Er liest nur Informationen und verändert nichts.
