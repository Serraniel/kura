# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later

## Command line

cli-usage =
    Aufruf: kura <Befehl>

    Befehle:
      doctor     Übersicht über dieses System für Problemberichte anzeigen
      orphans    Pakete auflisten, die nichts mehr braucht
      sbom       Liste aller installierten Software schreiben (CycloneDX)
      version    Version von kura anzeigen
cli-unknown-command = Unbekannter Befehl: { $command }
cli-missing-command = Kein Befehl angegeben.

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
