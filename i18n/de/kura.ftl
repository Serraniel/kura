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
