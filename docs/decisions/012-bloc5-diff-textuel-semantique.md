# ADR 012 — Bloc 5 : diff textuel et diff sémantique PLCopen

## Contexte

Le Bloc 4 (COM/UDE) est bloqué sur l'accès à une machine Control Expert. En
parallèle, le Bloc 3 avait livré le renderer SVG ladder et un diff visuel *fusionné*
(un seul SVG avec couleurs rouge/vert/jaune). Le Bloc 5 ferme les tâches de diff
restantes :

1. Diff textuel ligne à ligne pour les fichiers lisibles du STU (`.xso`, `.asm`)
2. Diff sémantique PLCopen : quels POUs, variables et réseaux ont changé
3. Vue deux colonnes dans le dashboard (avant / après séparés)

## Décisions

### 1. Diff textuel dans `stu-vcs` (crate `similar`)

**Crate retenue : `similar = "2"`** — bibliothèque Rust pure, bien maintenue, API
propre pour diff par lignes ou par caractères. Alternatives écartées :
- `dissimilar` : limité au diff caractère par caractère, peu adapté aux fichiers XML/ASM.
- Implémenter l'algorithme Myers soi-même : hors scope, `similar` est suffisant.

Deux fonctions publiques ajoutées dans `stu_vcs::diff` :

```rust
pub fn is_text_diffable(path: &str) -> bool   // true pour .xso et .asm
pub fn text_diff(old: &[u8], new: &[u8], path: &str) -> Option<String>
```

`text_diff` retourne `None` si le contenu est identique (`ratio() >= 1.0`) ou si
l'un des buffers n'est pas de l'UTF-8 valide — ce qui couvre les cas où un fichier
`.xso` serait corrompu ou absent. Aucune panique possible.

**Dans la CLI (`ioflow diff`)** : pour chaque `FileChange::Modified` dont le chemin
est text-diffable, les octets sont lus depuis l'object store et le patch unifié est
affiché avec coloration ANSI (vert `+`, rouge `-`, cyan `@`). Les binaires continuent
d'afficher uniquement la variation de taille.

### 2. Diff sémantique PLCopen (`plcopen::semantic_diff`)

Nouveau module `crates/plcopen/src/semantic_diff.rs`. Il travaille sur les types
`Project` déjà parsés — pas de re-parsing, pas d'I/O.

**Granularité retenue :**
- POUs : ajoutés / supprimés / modifiés (par nom)
- Variables : ajoutées / supprimées (toutes sections confondues : input, output,
  in_out, local, temp, external, global). La détection de *renommage* est intentionnellement
  absente — un rename apparaît comme remove + add, ce qui est juste et non ambigu.
- Réseaux LD : ajoutés / supprimés / modifiés (par `local_id` + clé de contenu
  sémantique, identique à celle du renderer diff du Bloc 3).

**Clé de comparaison réseau** : même logique que `content_key` dans
`renderer/diff.rs` — variable + négation + front pour les contacts/bobines, nom de
type pour les blocs. Positionnement ignoré (le déplacement d'un élément sans
changement de logique n'est pas un changement sémantique).

**Sérialisation** : `PlcDiff` et `PouDiff` dérivent `Serialize` → le backend peut
retourner du JSON brut (`GET /api/v1/diff/:h1/:h2/semantic`) ou convertir en HTML
(endpoint `POST /api/v1/render/plc-semantic-diff` stateless pour le dashboard htmx).

4 tests unitaires couvrent : var ajoutée, var supprimée, POU ajouté, aucun changement.

### 3. Vue deux colonnes (`render_diff_columns`)

La vue fusionnée (ADR 011) superpose les deux réseaux dans un seul SVG. Elle est
bonne pour un réseau dense mais peut être difficile à lire quand l'ancienne et la
nouvelle version ont des topologies très différentes.

Ajout de `render_diff_columns(a, b) -> (String, String)` dans `renderer/diff.rs` :
- SVG gauche = réseau A, avec les éléments *absents de B* en rouge et les éléments
  *modifiés* en jaune.
- SVG droit = réseau B, avec les éléments *absents de A* en vert et les modifiés
  en jaune.

Chaque SVG est rendu indépendamment via `render_network_colored` existant — pas de
logique dupliquée.

### 4. Nouveaux endpoints backend

Quatre nouveaux endpoints ajoutés dans `routes/ladder.rs` :

| Route | Nature | Retour |
|---|---|---|
| `POST /api/v1/render/ladder-diff-side` | stateless | HTML fragment (deux SVG grille) |
| `POST /api/v1/render/plc-semantic-diff` | stateless | HTML fragment coloré |
| `GET  /api/v1/diff/:h1/:h2/pou/:name/ladder/side` | DB-backed | HTML fragment |
| `GET  /api/v1/diff/:h1/:h2/semantic` | DB-backed | JSON `PlcDiff` |

Le HTML retourné par les endpoints stateless utilise une grille CSS inline
(`grid-template-columns: 1fr 1fr`) pour ne pas introduire de classe CSS globale
dans le dashboard.

### 5. Dashboard

Trois changements dans `web/templates/dashboard.html` :

1. **Toggle "Vue fusionnée / Côte à côte"** dans l'onglet Diff visuel — deux boutons
   qui modifient l'attribut `hx-post` du formulaire via `htmx.process()` pour
   basculer entre les deux endpoints sans rechargement.
2. **Onglet Diff sémantique** — formulaire avec deux textareas → `POST
   /api/v1/render/plc-semantic-diff` → inject HTML.
3. Styles CSS pour le diff sémantique (`.sem-diff`, `.sem-row.added/removed/modified`)
   cohérents avec les couleurs du renderer SVG.

## Conséquences

- `similar` est ajouté uniquement à `stu-vcs` (dépendance légère, pure Rust).
  Le workspace root n'est pas modifié (dépendance non partagée entre crates).
- La détection de renommage de variable reste absente : impossible sans heuristiques
  fragiles (distance de Levenshtein sur les noms, similarité de type…). À réévaluer
  si les utilisateurs remontent ce besoin.
- Le diff sémantique ne couvre pas FBD/SFC : `PouDiff.networks_*` est vide pour
  ces langages (`(vec![], vec![], vec![])` retourné par le match). Pas de panique,
  juste une information absente.
- Les endpoints stateless ne nécessitent pas de PostgreSQL — utiles pour tester
  le dashboard sur une machine sans DB.
- Le réseau résultant de `render_diff_columns` peut avoir des largeurs SVG
  différentes entre la colonne gauche et la colonne droite si les deux réseaux
  n'ont pas les mêmes dimensions. La grille CSS `1fr 1fr` absorbe cet écart.
