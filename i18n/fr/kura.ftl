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
      update     Installer toutes les mises à jour (avec un aperçu d’abord)
      install    Installer une application : kura install <nom>
      remove     Supprimer une application et ce dont elle seule avait besoin : kura remove <nom>
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

## Applying changes

preview-title = Ceci va changer :
confirm-update = Installer ces mises à jour ? [o/N]
news-title = Actualités d’Arch Linux concernant cette mise à jour :
news-unavailable = Les actualités d’Arch Linux n’ont pas pu être chargées. Veuillez lire https://archlinux.org/news/ avant de mettre à jour.
confirm-update-news = Avez-vous lu les actualités ci-dessus et voulez-vous installer les mises à jour ? [o/N]
confirm-install = Installer ceci ? [o/N]
confirm-remove = Supprimer ceci ? [o/N]
confirm-yes = o
apply-cancelled = Rien n’a été modifié.
apply-refreshing = Actualisation des listes de paquets…
apply-downloading = Téléchargement de { $file }
apply-checking = Vérification des paquets…
apply-installing = Installation de { $package } ({ $current }/{ $total })
apply-removing = Suppression de { $package } ({ $current }/{ $total })
apply-hook = Finalisation : { $name }
apply-keyring = Mise à jour des clés de signature en premier…
apply-cleanup = Suppression des éléments qui ne sont plus nécessaires…
apply-done = Terminé. La modification est enregistrée dans l’historique.
apply-changed = Quelque chose a changé depuis l’aperçu. Veuillez vérifier le nouvel aperçu.
apply-not-authorized = La modification n’a pas été autorisée. Soit le mot de passe n’a pas été saisi, soit ce compte n’a pas le droit d’installer des logiciels.
apply-locked = Un autre programme est en train d’installer des logiciels. Veuillez réessayer quand il aura terminé.
apply-busy = kura effectue déjà une autre modification. Veuillez attendre qu’elle soit terminée.
apply-failed = La modification a échoué : { $reason }
error-helper = Le service système de kura n’est pas disponible : { $reason }

## Tray

tray-uptodate = Tout est à jour
tray-updates =
    { $count ->
        [one] Une mise à jour est prête
       *[other] { $count } mises à jour sont prêtes
    }
tray-attention = Une mise à jour demande votre attention
tray-unknown = kura n’a pas encore recherché de mises à jour
tray-open-hint = Cliquez pour ouvrir kura.
notify-updates-body = Ouvrez kura pour voir ce qui change avant l’installation.
notify-attention-body = Ouvrez kura pour voir ce qu’il faut faire. Rien n’a été modifié.
notify-open = Ouvrir kura

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
