# TODO — ioflow

État au 2026-07-15. Organisé par bloc de priorité décroissante.

---

## Bloc 1 — VCS local (`stu-vcs` + CLI)

### Livré
- [x] `rustfmt.toml` à la racine (`max_width = 100`, `edition = "2021"`)
- [x] 21 tests d'intégration `stu-vcs` (`tests/vcs.rs`) couvrant init, objects,
      hash, tree, diff, archive, scénarios end-to-end
- [x] Fixture STU synthétique (ZIP créé programmatiquement dans les tests)

### Commandes manquantes
- [x] `ioflow status <f.stu>` : compare un STU local contre HEAD sans créer de commit
- [x] `ioflow config --name "Jean Dupont"` : écrire le nom auteur dans `.ioflow/config.toml`

### Qualité
- [x] Résolution de préfixe ambiguë → erreur explicite si plusieurs objets matchent
- [x] Message d'erreur lisible si le fichier `.stu` n'est pas un ZIP valide

---

## Bloc 2 — Backend réel

- [x] Brancher sqlx sur PostgreSQL (pool dans `AppState`)
- [ ] Implémenter les vraies routes :
  - [x] `POST /api/v1/agents/register` → UPSERT en base (org_id fourni par l'agent depuis sa config)
  - [x] `GET  /api/v1/jobs/poll`        → SELECT job queued + UPDATE running (transaction FOR UPDATE SKIP LOCKED)
  - [x] `POST /api/v1/jobs/{id}/status` → UPDATE résultat / diagnostics
  - `GET  /api/v1/projects`         → CRUD projets
- [ ] Ajouter service PostgreSQL dans le job CI (+ `SQLX_OFFLINE` ou DB de test)
- [ ] Auth : sessions + argon2 (inscription / connexion)
- [ ] Endpoint upload snapshot : `POST /api/v1/snapshots` (reçoit un `.stu` ou un tree)

---

## Bloc 3 — Rendu visuel ladder

### Livré (2026-07-17)
- [x] `crates/plcopen/src/renderer/svg.rs` : `LdNetwork` → SVG string
  - Rails gauche/droit, contact NO `[ ]` et NF `[/]`, bobine `( )` (pill shape)
  - Fils horizontaux (connexions) + fils en L (branches verticales)
  - Marqueurs de front (↑/↓), stockage bobine (S/R/M), bloc fonctionnel
  - 4 tests unitaires (NO, NF, dimensions, colorisation)
- [x] `crates/plcopen/src/renderer/diff.rs` : deux networks → SVG coloré
  - Rouge = supprimé, vert = ajouté, jaune = modifié, noir = inchangé
  - 4 tests unitaires (identiques, modifié, ajouté, supprimé)
- [x] Endpoints Axum :
  - `POST /api/v1/render/ladder`              → SVG stateless (formulaire htmx)
  - `POST /api/v1/render/ladder-diff`         → SVG diff stateless
  - `POST /api/v1/snapshots`                  → stockage XML PLCopen en DB
  - `GET  /api/v1/snapshots/{hash}/pous`      → liste des POUs (JSON)
  - `GET  /api/v1/snapshots/{hash}/pou/{name}/ladder?network=N` → SVG
  - `GET  /api/v1/diff/{h1}/{h2}/pou/{name}/ladder?network=N`  → SVG diff
- [x] Migration `0002_plcopen_snapshots.sql`
- [x] Dashboard htmx (`GET /`) : onglet Afficher + onglet Diff

---

## Bloc 4 — Agent + COM réel

- [ ] Tester UDE sur une machine Control Expert (confirmer disponibilité)
- [ ] Implémenter les vrais appels COM dans `com-bridge` (feature `com`) :
  - `open_project(path)` → `IProject::Open`
  - `build()` → `IProject::Build` + récupération des diagnostics
  - `export_plcopen(dest)` → export PLCopenXML via COM
  - `close_project()` → `IProject::Close`
- [ ] Export PLCopenXML automatique depuis l'agent (sans action manuelle)
- [ ] Self-hosted runner GitHub Actions sur la machine de test (pour CI COM réelle)

---

## Bloc 5 — Diff textuel et sémantique

- [ ] Diff textuel ligne à ligne sur `.xso` (crate `similar`)
- [ ] Diff textuel sur `.asm` (sections modifiées)
- [ ] Diff sémantique PLCopenXML :
  - Variables ajoutées / supprimées / renommées
  - POUs ajoutés / supprimés
  - Réseaux ladder ajoutés / supprimés / modifiés
- [ ] Vue diff visuel ladder dans le dashboard (deux colonnes rouge/vert)

---

## Backlog / Questions ouvertes

- [ ] Nom définitif du produit (actuellement "ioflow")
- [ ] Modèle de pricing (par agent ? par projet ? par build/mois ?)
- [ ] Hébergement cloud cible (Hetzner/OVH VPS, ou PaaS ?)
- [ ] Mode on-premise complet (pour clients réticents au cloud) ?
- [ ] Valider les CGU Schneider avant tout service commercial
- [ ] Entretiens utilisateurs : 5-10 intégrateurs / bureaux d'études
- [ ] Spike PLCopenXML sur Control Expert classique (vs. Machine Expert/CODESYS)
