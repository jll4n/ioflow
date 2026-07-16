# ADR 008 — Qualité VCS : préfixe ambigu et erreur STU lisible

## Contexte

Deux lacunes de qualité identifiées après les premières utilisations des commandes
`show`, `diff`, `restore` et `snapshot` :

1. **Préfixe ambigu** : `resolve_prefix` retournait silencieusement le premier
   objet trouvé si plusieurs hashes partageaient le même préfixe court. Comportement
   non-déterministe, indétectable par l'utilisateur.

2. **STU invalide** : passer un fichier qui n'est pas un ZIP à `snapshot` ou
   `status` produisait `"archive STU invalide : Invalid central directory signature"` —
   message en anglais issu de la crate `zip`, sans nom de fichier. L'utilisateur ne
   savait pas quel fichier était en cause.

## Décisions

### 1. Résolution de préfixe ambigu (`crates/cli/src/main.rs`)

`resolve_prefix` collecte désormais **tous** les matches avant de décider :

```
0 match  → "commit introuvable : {prefix}"
1 match  → hash complet (comportement inchangé)
n matchs → "préfixe ambigu : N commits commencent par '{prefix}' — soyez plus précis"
```

Aucun nouveau type d'erreur : la fonction retourne déjà `Box<dyn Error>`.

### 2. Erreur STU lisible (`crates/stu-vcs/src/error.rs` + `stu.rs`)

Nouveau variant `VcsError::InvalidStu(String)` :

```
'rapport.pdf' n'est pas un fichier STU valide (archive ZIP attendue)
```

Le chemin du fichier est capturé à l'ouverture (`ZipArchive::new`) via `map_err`.
L'erreur ZIP sous-jacente est ignorée : son message (anglais, technique) n'apporte
rien à l'utilisateur final.

Les erreurs ZIP survenant lors de la lecture des entrées (iteration) restent mappées
sur `VcsError::Zip` — elles indiquent une archive corrompue, pas un mauvais type de
fichier.

## Conséquences

- `ioflow show ab1` avec deux commits commençant par `ab1` → erreur explicite au lieu
  d'un résultat arbitraire.
- `ioflow snapshot mon_rapport.pdf` → message clair avec le nom du fichier.
- 1 test d'intégration ajouté (`test_invalid_stu_error` dans `tests/vcs.rs`).
- Le cas préfixe ambigu n'est pas testé automatiquement (il faudrait deux commits
  avec même préfixe dans un dépôt temporaire, ce qui est difficile à reproduire de
  façon déterministe sans contrôler les hashes) — couverture manuelle suffisante à
  ce stade.
