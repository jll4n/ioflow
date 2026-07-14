# ADR 004 — VCS local pour fichiers STU (`crates/stu-vcs`)

## Contexte

Les fichiers `.stu` (Control Expert / Unity Pro) sont des archives ZIP contenant
des fichiers en majorité chiffrés ou dans des formats propriétaires (voir ADR 002).
L'outillage de versionnage existant dans les bureaux d'études se limite à des copies
manuelles (`projet_v3_final_DEF.stu`), sans historique structuré ni diff lisible.

L'objectif de ce module est de fournir un VCS local autonome — sans dépendance à
Control Expert, à PostgreSQL ni à aucun service réseau — utilisable dès aujourd'hui
sur n'importe quelle machine Windows disposant du fichier `.stu`.

## Décision

### Modèle objet (inspiré de Git, simplifié)

Trois types d'objets, tous stockés dans le même store contenu-adressé :

```
Blob    = contenu brut d'un fichier (bytes)
Tree    = JSON { "chemin": "hash_blob", ... }   → BTreeMap pour ordre déterministe
Commit  = JSON { parent, tree, message, author, timestamp }
```

Le hash de chaque objet est son **SHA-256** encodé en hexadécimal (64 chars).
L'adressage par contenu garantit la déduplication : un fichier STU dont 90 % des
blobs sont inchangés ne stocke que les blobs modifiés.

### Layout du dépôt

```
.ioflow/
  HEAD                        # "ref: refs/heads/main\n"
  config.toml                 # [user] name = "..."
  refs/
    heads/
      main                    # hash hex du dernier commit
  objects/
    ab/
      cdef1234...             # blob, tree ou commit (même store)
```

### Crate `stu-vcs` (bibliothèque)

| Module | Responsabilité |
|---|---|
| `hash.rs` | Type `Hash = String`, `hash_bytes()` (SHA-256), `short()` (7 chars) |
| `objects.rs` | `ObjectStore` : écriture idempotente, lecture par hash |
| `tree.rs` | `Tree` : `BTreeMap<String, Hash>` + sérialisation JSON |
| `commit.rs` | `Commit` : parent, tree, message, author, timestamp |
| `repo.rs` | `Repo::init()`, `Repo::open()` (remonte l'arborescence), `head()`, `set_head()`, `author()` |
| `stu.rs` | `StuArchive::open()` (extraction ZIP), `StuArchive::write()` (reconstruction) |
| `diff.rs` | `diff_trees()` → `Vec<FileChange>`, `file_label()` (étiquette par extension) |

`stu-vcs` est une bibliothèque pure : pas d'I/O réseau, pas de tokio, pas de
dépendance à Control Expert. Testable et réutilisable par le CLI et le backend.

### Crate `cli` — binaire `ioflow`

Implémente 6 commandes via `clap` (derive API) :

| Commande | Action |
|---|---|
| `ioflow init` | Crée `.ioflow/` avec HEAD, config, refs/, objects/ |
| `ioflow snapshot <f.stu> [-m "..."] [--export f.xml]` | Extrait le STU, hashe chaque fichier, crée Tree + Commit, met à jour HEAD |
| `ioflow log` | Remonte la chaîne de commits depuis HEAD, affiche hash court + date + auteur + message |
| `ioflow show <hash>` | Détail d'un commit : métadonnées + liste de tous les fichiers avec leur type |
| `ioflow diff <h1> <h2>` | Compare deux trees, affiche `+`/`-`/`~` par fichier avec taille avant/après |
| `ioflow restore <hash> -o f.stu` | Recrée le STU depuis les blobs du snapshot (exclut `program.xml`) |

La résolution de préfixe (ex : `a3f2c1` → hash complet) est implémentée dans
`resolve_prefix()` par parcours de `.ioflow/objects/`.

### Gestion de l'export PLCopenXML

`ioflow snapshot` accepte `--export fichier.xml`. Le XML est stocké comme un blob
ordinaire sous la clé `program.xml` dans le Tree. Cette clé est exclue lors de
`ioflow restore` (elle n'est pas un fichier natif du STU).

Ce mécanisme permet d'associer un export PLCopenXML à un snapshot sans modifier
l'archive STU, et servira de base au diff sémantique et au rendu SVG ladder
(voir blocs 3 et 5 de la roadmap).

### Diff par type de fichier

Le diff entre deux commits est aujourd'hui **au niveau fichier** (hash changed = modified).
Chaque changement affiche une étiquette selon l'extension :

| Extension | Étiquette |
|---|---|
| `.xso` | XML paramètres |
| `.xpdf` | XML chiffré Schneider |
| `.db` | base propriétaire eXc |
| `.asm` | assembleur généré |
| `.apb / .apd / .apx` | binaire compilé |
| `.ctx` | contexte binaire |
| `.odb` | base objets |
| autres | binaire |

Un diff textuel ligne à ligne sur les fichiers `.xso` et `.asm` est la prochaine
itération naturelle (crate `similar` ou implémentation maison).

## Dépendances ajoutées

| Crate | Version | Usage |
|---|---|---|
| `sha2` | 0.10 | Calcul SHA-256 |
| `hex` | 0.4 | Encodage hexadécimal |
| `zip` | 2 | Lecture/écriture archives `.stu` |
| `clap` | 4 (derive) | Parser CLI |

## Ce qui reste à faire (hors scope de cet ADR)

- Diff textuel sur `.xso` et `.asm` (crate `similar`)
- `ioflow status <f.stu>` : comparer un STU local contre HEAD sans créer de commit
- Résolution de préfixe ambiguë (plusieurs objets matchent → erreur explicite)
- Tests unitaires `stu-vcs` (init, snapshot, log, diff sur fixtures)
- Intégration au backend : uploader un snapshot via l'API pour stockage cloud

## Conséquences

- Un bureau d'études peut versionner ses projets `.stu` localement dès maintenant,
  sans Control Expert installé et sans accès réseau.
- Le store contenu-adressé assure la déduplication : un projet de 2 Mo dont un seul
  `.db` change ne stocke que les blobs modifiés (~quelques Ko).
- La chaîne de commits est immuable par construction (modifier un commit changerait
  son hash, cassant la référence parent).
- L'ajout de `program.xml` dans le Tree est transparent pour `restore` : le STU
  reconstruit est identique à l'original.
