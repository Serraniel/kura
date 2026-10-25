# SPDX-FileCopyrightText: 2026 The kura contributors
# SPDX-License-Identifier: GPL-3.0-or-later
# Status: machine-assisted, review wanted.

## Command line

cli-usage =
    Utilisation : kura <commande>

    Commandes :
      doctor     Afficher un résumé de ce système pour les rapports de problème
      orphans    Lister les paquets dont plus rien n’a besoin
      sbom       Écrire la liste de tous les logiciels installés (CycloneDX)
      version    Afficher la version de kura
cli-unknown-command = Commande inconnue : { $command }
cli-missing-command = Aucune commande indiquée.

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
