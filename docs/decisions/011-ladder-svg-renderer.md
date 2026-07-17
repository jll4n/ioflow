# ADR 011 — Renderer SVG ladder et dashboard htmx

## Contexte

Le crate `plcopen` parsait déjà le format PLCopenXML en types Rust (contacts, bobines,
blocs, rails). L'étape suivante (Bloc 3 du backlog) est de rendre ces données
visuellement dans un navigateur sous forme de SVG, et de proposer un diff visuel
entre deux versions d'un réseau.

Deux sous-problèmes :
1. **Rendu unitaire** : `LdNetwork → SVG` avec tous les symboles IEC 61131-3 LD.
2. **Rendu diff** : comparer deux `LdNetwork` et mettre en couleur les différences.

## Décision

### Système de coordonnées

PLCopen stocke les positions en unités abstraites (`position x/y`) et des offsets
relatifs (`relPosition`) pour les points de connexion. Dans les exports Control
Expert observés, les valeurs typiques sont :
- x : 0, 50, 110, 200, 250 (unités larges)
- y : 0, 1, 2 (petites valeurs, chaque unité = une "rangée")

Facteurs d'échelle retenus : **SX = 3** (px par unité x), **SY = 30** (px par
unité y). Avec ces valeurs, un réseau simple (2 contacts + 1 bobine) produit un SVG
d'environ 800 × 100 px, lisible sans zoom.

Les symboles (contacts, bobines) sont dessinés à **taille fixe** (40 × 20 px)
positionnés sur le bord gauche de leur coordonnée PLCopen. Les fils aboutissent
à ce bord gauche (entrée) et partent du bord droit (sortie = gauche + SYMBOL_W).
Cela déconnecte la taille visuelle de la `relPosition` PLCopen tout en conservant
l'alignement vertical via `rel_position.y`.

### Symboles retenus

| Élément | Rendu SVG |
|---|---|
| `LeftPowerRail` / `RightPowerRail` | `<rect>` plein, largeur 5 px, hauteur `height * SY` |
| Contact NO | `<rect>` contour noir, fond blanc |
| Contact NF | idem + `<line>` diagonale intérieure |
| Contact front ↑/↓ | marqueur texte en haut à droite du rect |
| Bobine (normale) | `<rect>` arrondi (`rx = W/4, ry = H/2`) → forme pill = `( )` |
| Bobine SET/RESET | pill + lettre intérieure S/R/M |
| Bloc fonctionnel | `<rect>` avec nom de type (gras), instance (italique), pins |
| Fils | `<line>` si même y ; `<polyline>` en L si y différents |

La bobine pill (`rx = 10, ry = 10` sur 40 × 20) est suffisamment distincte
d'un contact rectangulaire pour être reconnue d'un coup d'œil.

### API du renderer

```
crates/plcopen/src/renderer/
  mod.rs      → ElemColor (Normal / Added / Removed / Modified)
               helper element_local_id()
  svg.rs      → render_network(&LdNetwork) → String
               render_network_colored(&LdNetwork, &dyn Fn(u32) → ElemColor) → String
  diff.rs     → render_diff(&LdNetwork, &LdNetwork) → String
```

`render_network_colored` est l'API centrale : elle accepte une closure par `localId`
pour déterminer la couleur de chaque élément. Le renderer diff et les futures
annotations (ex. : mise en évidence depuis le dashboard) l'utilisent toutes.

### Stratégie de diff

La comparaison se fait par `localId` + **clé de contenu sémantique** (variable,
négation, front, type de stockage) — indépendante de la position. Les éléments
supprimés (présents dans A, absents de B) sont ajoutés au réseau fusionné à leur
position d'origine dans A et colorisés en rouge.

Couleurs retenues : vert `#16a34a` (ajouté), rouge `#dc2626` (supprimé),
jaune/ambre `#d97706` (modifié), noir `#1a1a1a` (inchangé).

### Endpoints Axum

Deux catégories :

**Stateless (sans DB) — pour le dashboard htmx :**
- `POST /api/v1/render/ladder` — reçoit un formulaire `{xml, pou, network}`, retourne SVG
- `POST /api/v1/render/ladder-diff` — reçoit `{xml_a, xml_b, pou, network}`, retourne SVG diff

**DB-backed — pour l'intégration agent :**
- `POST /api/v1/snapshots` — stocke un XML PLCopen indexé par `commit_hash` (hash stu-vcs)
- `GET  /api/v1/snapshots/{hash}/pous` — liste JSON des POUs et leur langage
- `GET  /api/v1/snapshots/{hash}/pou/{name}/ladder?network=N` — SVG
- `GET  /api/v1/diff/{h1}/{h2}/pou/{name}/ladder?network=N` — SVG diff

La migration `0002_plcopen_snapshots.sql` crée la table `plcopen_snapshots
(commit_hash TEXT UNIQUE, xml_content TEXT)`.

Le backend sert le dashboard via `GET /` avec `include_str!` au moment de la
compilation (pas de fichier servi à la volée, pas de dépendance `ServeDir`).

### Dashboard

Page HTML minimaliste avec htmx 2.x (CDN). Deux onglets :
- **Afficher** : textarea XML + champ POU + index réseau → `POST /api/v1/render/ladder`
  → SVG injecté via `hx-swap="innerHTML"`
- **Diff** : deux textareas + champ POU → `POST /api/v1/render/ladder-diff` → SVG diff
  avec légende des couleurs

## Conséquences

- Le crate `plcopen` est toujours une bibliothèque pure (pas de tokio, pas d'I/O).
  Le renderer ne fait que construire une `String`.
- Le backend reçoit `plcopen` comme dépendance workspace ; il parse et rend à la volée
  (pas de cache SVG pour l'instant — acceptable car les réseaux sont petits).
- Les endpoints DB-backed nécessitent un PostgreSQL avec la migration 0002 appliquée.
  Les endpoints stateless fonctionnent sans DB (utiles pour les tests manuels).
- Le positionnement des symboles suppose que les coordonnées PLCopen x sont "larges"
  (ordre de grandeur 50–250) et y sont "petites" (0–10). Si un export réel de
  Control Expert utilise des valeurs différentes, les constantes SX/SY devront
  être ajustées — elles sont isolées en tête de `svg.rs`.
- Le rendu FBD et SFC reste en stub : seul le LD est rendu visuellement.
