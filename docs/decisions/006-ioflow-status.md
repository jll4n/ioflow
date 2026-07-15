# ADR 006 — Commande `ioflow status`

## Contexte

Après l'implémentation des commandes `snapshot`, `log`, `show`, `diff` et `restore`,
le workflow VCS avait un angle mort : pour savoir ce qui avait changé dans un fichier
`.stu` depuis le dernier snapshot, l'utilisateur devait soit faire un `snapshot` (polluer
l'historique), soit faire un `diff` en cherchant manuellement le hash de HEAD dans `log`.

L'équivalent `git status` manquait — la commande la plus utilisée au quotidien.

## Décision

### Implémentation dans `crates/cli/src/main.rs`

Ajout du variant `Status { stu: PathBuf }` dans l'enum `Commands` clap.

### Principe clé : calcul éphémère, store non modifié

`cmd_status` calcule les hashes du STU courant **en mémoire uniquement**, sans rien
écrire dans `.ioflow/objects/`. C'est la différence fondamentale avec `snapshot` :

```
snapshot : ouvre STU → hash → ÉCRIT les blobs → crée Tree + Commit → met à jour HEAD
status   : ouvre STU → hash → compare avec HEAD tree → affiche → ne touche à rien
```

Ce choix est délibéré : `status` est une commande d'inspection, pas de modification.
Appeler `status` dix fois de suite ne doit avoir aucun effet de bord.

### Réutilisation de `diff_trees`

Le diff est calculé via `diff_trees(&head_tree, &current_tree)` — exactement la même
fonction que `ioflow diff`. Aucune logique de comparaison dupliquée.

La taille "avant" est lue depuis le store (`repo.objects.read(old_hash)`), la taille
"après" directement depuis le buffer en mémoire (`archive.files[path].len()`).

### Cas HEAD = None

Si aucun snapshot n'existe encore, `status` affiche un message informatif et quitte
proprement plutôt que de retourner une erreur :

```
Aucun snapshot — lancez 'ioflow snapshot' d'abord.
```

Ce comportement suit la convention de `git status` sur un repo vide.

### Sortie type

```
HEAD     a3f2c1 — Ajout convoyeur B
Fichier  mon_projet.stu

  ~ ASPROG.db           [base propriétaire eXc]  180.0 KB → 183.2 KB
  ~ Project_Settings.xso [XML paramètres]  2.1 KB → 2.1 KB
  = Project_Definition.xpdf  [XML chiffré Schneider] — inchangé

2 modifié(s), 0 ajouté(s), 0 supprimé(s) — 'ioflow snapshot' pour committer
```

Le hint en fin de sortie (`'ioflow snapshot' pour committer`) guide l'utilisateur
vers l'action suivante sans documentation externe.

## Conséquences

- Le workflow complet est désormais utilisable sans friction :
  `status` → inspecter → `snapshot` → `log` → `diff` → `restore`
- Zéro nouvelle dépendance, zéro nouveau module dans `stu-vcs`.
- La commande `status` peut être étendue pour afficher un diff textuel des `.xso`
  et `.asm` dès que la crate `similar` sera intégrée (bloc 5 de la roadmap).
