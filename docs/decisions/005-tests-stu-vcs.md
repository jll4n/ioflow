# ADR 005 — Stratégie de tests pour `stu-vcs` et configuration rustfmt

## Contexte

Le crate `stu-vcs` constitue le cœur fonctionnel du produit : store contenu-adressé,
modèle commit, parsing ZIP, diff. À la livraison initiale des 6 commandes CLI, aucun
test automatisé ne couvrait ces composants. Un bug dans `objects.rs` ou `repo.rs`
passerait inaperçu jusqu'en production.

Par ailleurs, chaque push déclenchait des allers-retours CI sur `cargo fmt --all --check`
car rustfmt reformatait le code différemment selon des règles implicites (largeur de
ligne, indentation des struct fields…). Aucune configuration explicite ne fixait les
paramètres de formatage.

## Décision

### 1. Tests d'intégration dans `crates/stu-vcs/tests/vcs.rs`

Choix d'un fichier de tests d'intégration unique plutôt que des `#[cfg(test)]` dispersés
dans chaque module, pour deux raisons :

- Les tests accèdent à l'API publique du crate (comme le ferait le CLI) — ils valident
  le contrat externe, pas les détails internes.
- Un fichier unique facilite la lecture de bout en bout du scénario complet
  (init → snapshot → diff → restore).

**21 tests répartis en 7 groupes :**

| Groupe | Nb | Ce qui est testé |
|---|---|---|
| `Repo / init` | 5 | Structure `.ioflow/`, erreur si déjà init, remontée d'arborescence, `NotARepo`, HEAD vide |
| `ObjectStore` | 3 | Round-trip write/read, idempotence, erreur sur hash inconnu |
| `Hash` | 3 | Déterminisme SHA-256, collision, `short()` à 7 chars |
| `Tree` | 2 | Sérialisation JSON round-trip, ordre déterministe (BTreeMap) |
| `Diff` | 4 | 4 types de changement, tri par chemin, trees identiques, `file_label()` |
| `StuArchive` | 2 | Extraction ZIP, write + open round-trip |
| **Intégration** | **5** | Snapshot complet, chaîne de commits, déduplication blobs, restore, diff end-to-end |

**Fixture STU synthétique :** les tests créent des archives ZIP en mémoire via la
fonction helper `make_stu_bytes(&[(&str, &[u8])])`. Pas de dépendance au vrai projet
Schneider (`stuexample/`), ce qui évite d'embarquer des données propriétaires dans les
tests et rend la suite rapide (~ms par test).

### 2. Dépendance de développement : `tempfile`

`tempfile = "3"` ajouté en `[dev-dependencies]` de `stu-vcs`. Chaque test crée un
`TempDir` isolé qui se supprime automatiquement à la fin du test. Alternatives écartées :

- `std::env::temp_dir()` + cleanup manuel → risque de fuite si le test panique
- Dossier fixe en dur → collisions entre tests en parallèle

### 3. `rustfmt.toml` à la racine

```toml
max_width = 100
edition   = "2021"
```

Explicite les deux paramètres qui causaient des diffs CI : la largeur de ligne (100 au
lieu des 80 par défaut que rustfmt applique dans certains contextes) et l'édition Rust.
Désormais `cargo fmt --all` en local produit exactement ce que la CI vérifie.

## Tests de régression couverts

| Scénario | Test |
|---|---|
| `init` crée la bonne structure | `init_cree_structure_ioflow` |
| Double `init` échoue | `init_echoue_si_deja_initialise` |
| `open` remonte l'arborescence | `open_remonte_larborescence` |
| Blob écrit puis relu identique | `write_et_read_round_trip` |
| Même blob → un seul objet sur disque | `write_est_idempotent` |
| SHA-256 déterministe | `hash_est_deterministe` |
| Tree JSON stable (BTreeMap) | `tree_ordre_deterministe` |
| `diff_trees` : 4 types | `diff_detecte_tous_les_types_de_changement` |
| `diff_trees` : résultats triés | `diff_retourne_resultats_tries_par_chemin` |
| `file_label` : toutes extensions | `file_label_retourne_bonne_etiquette` |
| STU ZIP round-trip | `stu_write_et_open_round_trip` |
| Snapshot stocke le bon tree | `snapshot_cree_commit_avec_bon_tree` |
| Chaîne parent → enfant | `deux_snapshots_chaine_de_commits` |
| Déduplication blobs identiques | `blobs_inchanges_sont_dedupliques` |
| Restore → fichiers identiques octet par octet | `restore_reproduit_fichiers_originaux` |
| Diff end-to-end sur deux snapshots | `diff_entre_deux_snapshots` |

## Conséquences

- La CI exécute désormais 21 tests `stu-vcs` + 4 tests `plcopen` = 25 tests au total.
- Toute régression dans le cœur VCS (hash, store, tree, commit, diff) est détectée
  immédiatement avant merge.
- La fixture synthétique peut être étendue pour tester `ioflow status` et
  `ioflow config` dès que ces commandes seront implémentées.
- `rustfmt.toml` élimine les allers-retours CI/fmt — `cargo fmt --all` en local
  suffit avant chaque push.
