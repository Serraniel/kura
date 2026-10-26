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
      mirrors    Choisir des serveurs de téléchargement rapides
      orphans    Lister les paquets dont plus rien n’a besoin
      sbom       Écrire la liste de tous les logiciels installés (CycloneDX)
      search     Chercher des applications dans les dépôts et l'AUR : kura search <mot>
      update     Installer toutes les mises à jour (avec un aperçu d’abord)
      install    Installer une application : kura install <nom>
      remove     Supprimer une application et ce dont elle seule avait besoin : kura remove <nom>
      security   Afficher les problèmes de sécurité connus des logiciels installés
      settings   Afficher ou modifier les réglages : kura settings aur on|off, kura settings aur-wait <jours>
      snapshots  Afficher l'état des instantanés ; kura snapshots setup ajoute ce qui manque
      undo       Revenir à un état antérieur : kura undo [numéro]
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
change-blocked = La modification ne peut pas être préparée :
problem-unsatisfied = { $target } a besoin de { $dependency }, qui n’est plus disponible.
problem-conflict = { $first } et { $second } ne peuvent pas être installés ensemble.
problem-arch = Conçu pour un autre type de processeur : { $packages }
problem-not-found = Aucun paquet nommé { $name } n’a été trouvé.
problem-not-restorable = kura ne peut pas récupérer { $name } { $version } : il provient de l’AUR ou a été compilé sur cet ordinateur, et le fichier n’est plus dans le cache des paquets.
undo-title = Retour à l’état n° { $id } du { $when } :
undo-limits = Les paramètres et les fichiers personnels ne sont pas modifiés. La prochaine mise à jour ramènera les versions plus récentes : utilisez ceci pour contourner un problème en attendant un correctif.
undo-nothing = C’est déjà l’état actuel.
undo-unknown = Il n’existe pas d’état enregistré n° { $id }. kura history liste les états enregistrés.
confirm-undo = Revenir à cet état ? [o/N]
mirrors-running = Test des serveurs de téléchargement proches…
mirrors-done = { $count } serveurs de téléchargement rapides sont maintenant utilisés. L’ancienne liste a été conservée sous { $backup }.
solver-found =
    { $count ->
        [one] kura a trouvé une solution : un ancien élément qu’aucune application n’utilise plus bloque la mise à jour. Il est supprimé, tout le reste est mis à jour.
       *[other] kura a trouvé une solution : { $count } anciens éléments qu’aucune application n’utilise plus bloquent la mise à jour. Ils sont supprimés, tout le reste est mis à jour.
    }
solver-tested = kura a testé cette solution sans rien modifier.
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
apply-building = Compilation de { $name } ({ $current }/{ $total })
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

## Security

security-one-app = { $app } a un problème de sécurité connu
security-more-apps =
    { $count ->
        [one] { $app } et une autre application ont un problème de sécurité connu
       *[other] { $app } et { $count } autres applications ont un problème de sécurité connu
    }
security-system = Le système a un problème de sécurité connu
security-fix = Une mise à jour le corrige. Veuillez l’installer bientôt.
security-nofix = Il n’existe pas encore de correctif. kura vérifie chaque jour et vous prévient dès qu’il y en a un.
security-none = Aucun problème de sécurité connu.
security-running = Recherche des problèmes de sécurité connus…
security-unavailable = La liste des problèmes de sécurité connus n’a pas pu être chargée.
security-detail = { $package } { $installed } : { $kind }
security-fixed-in = Corrigé dans { $version }
security-used-by = Utilisé par : { $apps }
severity-low = risque faible
severity-medium = risque moyen
severity-high = risque élevé
severity-critical = risque critique

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
nav-updates = Mises à jour
gui-review = Voir et installer
gui-check-now = Rechercher des mises à jour maintenant
gui-install = Installer
gui-install-news = J’ai lu les actualités. Installer
gui-read-news = Lire l’annonce
gui-go-back = Revenir à cet état
gui-cancel = Annuler
gui-updates-intro = Voici tout ce qui va changer. Rien ne se passe avant que vous cliquiez sur Installer.
gui-tray-mono = Icône monochrome dans la zone de notification
gui-tray-mono-hint = S’accorde avec les autres icônes du tableau de bord.
gui-aur = Logiciels de l'AUR
gui-aur-hint = L'AUR contient des logiciels partagés par d'autres utilisateurs. Arch Linux ne les vérifie pas. kura les compile sur cet ordinateur dans un environnement séparé et restreint.
gui-aur-wait-hint = Les nouvelles versions de l'AUR attendent ce délai avant que kura les installe, pour que les problèmes apparaissent d'abord. Installer une application à la main prend toujours la version la plus récente.
gui-last-check = Dernière vérification : { $when }.
nav-health = État du système
nav-software = Logiciels
health-cpu = Processeur
health-gpu = Carte graphique
health-memory = Mémoire
health-disk = Stockage
health-disk-used = { $used } utilisés sur { $total }
health-cache = Anciens fichiers de mise à jour : { $size }
health-kernels = Noyaux
health-kernel-running = en cours d’utilisation
health-suggestions = Suggestions
health-snapshots = Instantanés
snapshots-on = Pris avant chaque mise à jour par { $tool }
snapshots-off = Non configurés
snapshots-partial = Partiellement configurés
snapshots-unsupported = Impossible avec ce système de fichiers
snapshots-missing = Il manque :
snapshots-step-install = Installer { $package }
snapshots-step-config = Créer une configuration snapper pour /
snapshots-step-enable = Activer { $unit }
snapshots-setup-hint = Pour ajouter ce qui manque : kura snapshots setup
snapshots-nothing = Rien à configurer.
preview-actions = Réglages modifiés :
confirm-snapshots = Configurer les instantanés ? [o/N]
suggest-backup-kernel = Installer un noyau de secours
suggest-backup-kernel-text = Si une mise à jour empêche un jour l’ordinateur de démarrer, le noyau de secours peut être choisi dans le menu de démarrage.
suggest-snapshots = Activer les instantanés automatiques
suggest-snapshots-text = Un instantané enregistre l'état du système avant chaque mise à jour, pour pouvoir revenir en arrière si quelque chose ne fonctionne plus. kura conserve ce qui est déjà configuré et n'ajoute que ce qui manque.
suggest-mirrors = Choisir des serveurs de téléchargement plus rapides
suggest-mirrors-text = kura teste les serveurs proches et utilise les plus rapides.
suggest-cleanup =
    { $count ->
        [one] Un paquet n’est plus nécessaire
       *[other] { $count } paquets ne sont plus nécessaires
    }
suggest-cleanup-text = Ils seront supprimés lors de la prochaine mise à jour.
search-placeholder = Rechercher des applications, par exemple « photo » ou « musique »
search-none = Aucune application trouvée. Essayez un autre mot.
search-repos = Depuis les dépôts :
search-aur = Depuis l'AUR (partagé par la communauté, compilé sur cet ordinateur) :
search-aur-failed = Impossible de chercher dans l'AUR : { $reason }
aur-votes = { $count ->
    [one] { $count } vote
   *[other] { $count } votes
}
aur-new = nouveau
aur-orphaned = sans mainteneur
aur-out-of-date = signalé comme obsolète
aur-mixed = Certains viennent de l'AUR. Installez-les séparément : kura install { $names }
aur-build-list = Compilé sur cet ordinateur depuis l'AUR :
aur-notice = Les paquets de l'AUR sont partagés par d'autres utilisateurs et ne sont pas vérifiés par Arch Linux. kura les compile dans un environnement séparé et restreint.
confirm-aur = Compiler et installer ceci ? [o/N]
aur-off = { $names } n'est pas dans les dépôts et l'AUR est désactivé. Pour l'activer : kura settings aur on
aur-check-failed = Impossible de vérifier les mises à jour de l'AUR : { $reason }
check-aur = Depuis l'AUR : { $ready } prêtes, { $waiting } en attente
aur-updates-title = Mises à jour depuis l'AUR :
aur-waiting = { $count ->
    [one] Une mise à jour de l'AUR attend
   *[other] { $count } mises à jour de l'AUR attendent
} d'avoir { $days } jours, pour que les problèmes des nouvelles versions apparaissent d'abord : { $names }
confirm-aur-update = Compiler et installer ces mises à jour ? [o/N]
settings-aur = AUR
settings-aur-wait = Attente avant les mises à jour de l'AUR
settings-on = activé
settings-off = désactivé
settings-days = { $days ->
    [one] { $days } jour
   *[other] { $days } jours
}
settings-saved = Enregistré.
settings-usage = Utilisation : kura settings [aur on|off] [aur-wait <0–14>]
gui-installed = Installé
gui-remove = Supprimer
gui-install-title = Installer { $name }
gui-remove-title = Supprimer { $name }
gui-setup = Configurer
gui-details = Détails
nav-history = Historique
nav-settings = Paramètres
overview-subtitle = Un bref aperçu de cet ordinateur.
overview-refresh = Actualiser
