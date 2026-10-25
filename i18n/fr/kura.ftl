# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
# Status: machine-assisted, review wanted.

## Command line

cli-usage =
    Utilisation : kura <commande>

    Commandes :
      check      Rechercher des mises à jour et montrer ce qui changerait
      doctor     Afficher un résumé de ce système pour les rapports de problème
      history    Afficher les états enregistrés du système
      orphans    Lister les paquets dont plus rien n’a besoin
      sbom       Écrire la liste de tous les logiciels installés (CycloneDX)
      version    Afficher la version de kura
cli-unknown-command = Commande inconnue : { $command }
cli-missing-command = Aucune commande indiquée.

## check

check-running = Recherche de mises à jour…
check-none = Tout est à jour.
check-found =
    { $count ->
        [one] Une mise à jour est disponible.
       *[other] { $count } mises à jour sont disponibles.
    }
check-group-apps = Applications ({ $count })
check-group-system = Composants du système ({ $count })
check-group-replaced = Remplacés pendant la mise à jour ({ $count })
check-group-cleanup = Plus nécessaires ({ $count })
check-sizes = Téléchargement : { $download } · Espace disque : { $delta }
check-preview-only = Rien n’a été modifié. L’installation des mises à jour arrivera dans une version ultérieure de kura.
check-blocked = La mise à jour ne peut pas être préparée :
problem-unsatisfied = { $target } a besoin de { $dependency }, qui n’est plus disponible.
problem-conflict = { $first } et { $second } ne peuvent pas être installés ensemble.
problem-arch = Conçu pour un autre type de processeur : { $packages }
problem-not-found = Aucun paquet nommé { $name } n’a été trouvé.
error-check = Impossible de rechercher des mises à jour : { $reason }

## history

history-title = États enregistrés du système
history-empty = Rien n’a encore été enregistré. kura enregistre désormais chaque modification.
history-today = aujourd’hui
history-yesterday = hier
history-days-ago =
    { $days ->
        [one] il y a un jour
       *[other] il y a { $days } jours
    }
history-installed =
    { $count ->
        [one] { $count } installé
       *[other] { $count } installés
    }
history-upgraded = { $count } mis à jour
history-downgraded =
    { $count ->
        [one] { $count } revenu à une version antérieure
       *[other] { $count } revenus à une version antérieure
    }
history-removed =
    { $count ->
        [one] { $count } supprimé
       *[other] { $count } supprimés
    }
history-start = Point de départ : { $count } paquets
history-outside = (modifié en dehors de kura)
list-separator = {", "}

## doctor

doctor-title = Résumé du système
doctor-distro = Distribution
doctor-packages = Paquets installés
doctor-explicit = Installés volontairement
doctor-foreign = Depuis l’AUR ou compilés localement
doctor-orphans = Plus nécessaires
doctor-language = Langue

## orphans

orphans-none = Rien à nettoyer. Tous les paquets installés sont encore nécessaires.
orphans-found =
    { $count ->
        [one] Un paquet n’est plus nécessaire :
       *[other] { $count } paquets ne sont plus nécessaires :
    }
orphans-hint = kura proposera de les supprimer après la prochaine mise à jour.

## sbom

sbom-written = Liste des logiciels enregistrée dans { $path }.

## Errors

error-database = Impossible de lire la base de données des paquets : { $reason }
error-write = Impossible d’écrire { $path } : { $reason }

## Window

nav-overview = Vue d’ensemble
nav-history = Historique
nav-settings = Paramètres
overview-subtitle = Un bref aperçu de cet ordinateur.
overview-refresh = Actualiser
prototype-note = Ceci est un premier prototype. Il lit uniquement des informations et ne modifie rien.
