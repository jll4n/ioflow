# ioflow

> CI/CD pour projets d'automatisme Schneider Electric (Control Expert / Unity Pro)

**Statut : spike / MVP en cours — pas encore production-ready.**

---

## Problème adressé

Les projets automates Modicon (M340, M580, Quantum, Premium) sont aujourd'hui versionnés
à la main : copies de fichiers, noms du type `projet_v3_final_DEF.stu`, sans diff lisible,
sans compilation automatique, sans traçabilité des changements avant un déploiement sur site.

ioflow vise à apporter aux bureaux d'études et intégrateurs une chaîne CI/CD adaptée à
Control Expert : compilation automatique à chaque commit, rapport d'erreurs/warnings, et
diff structurel entre deux versions d'un projet.

---

## Architecture

```
┌─────────────────────────────┐
│   Poste client (Windows)    │
│                             │
│  ┌─────────────────────┐    │
│  │ Agent (x64)         │    │
│  │  └─ com-bridge (x86)│◄───┼──── pull jobs (HTTPS)
│  │     └─ COM/UDE      │    │
│  └─────────────────────┘    │
└─────────────────────────────┘
              ▲
              │
┌─────────────┴───────────────┐
│  Backend cloud (Axum)       │
│  - API REST                 │
│  - File de jobs             │
│  - Rapports / historique    │
└─────────────┬───────────────┘
              │
┌─────────────┴───────────────┐
│  PostgreSQL                 │
│  orgs, users, projects,     │
│  agents, jobs, diagnostics  │
└─────────────────────────────┘
```

Principe clé : le backend cloud n'exécute jamais Control Expert. Les agents tournent
chez les clients, sur des machines sous licence, et viennent chercher du travail en
polling (modèle "self-hosted runner", à la GitHub Actions).

### Pourquoi un processus x86 séparé (`com-bridge`) ?

L'API COM/UDE de Control Expert expose des interfaces 32 bits uniquement. L'agent
principal tourne en x64 (pour bénéficier de l'écosystème Rust moderne). La solution
retenue est de spawner un sous-processus x86 dédié (`com-bridge.exe`) et de
communiquer avec lui via JSON newline-delimited sur stdin/stdout. Voir
[docs/decisions/001-com-bridge-x86.md](docs/decisions/001-com-bridge-x86.md).

---

## Crates

| Crate | Rôle |
|---|---|
| `shared` | Types communs agent ↔ backend : `Job`, `JobStatus`, `JobResult`, `Diagnostic`, protocoles HTTP et IPC |
| `backend` | API Axum + sqlx : enregistrement d'agents, polling de jobs (`FOR UPDATE SKIP LOCKED`), mise à jour de statut/diagnostics, endpoints ladder/diff |
| `agent` | Daemon x64 : registration au démarrage, polling toutes les 5 s, orchestration du `com-bridge`, remontée des résultats |
| `com-bridge` | Binaire x86 : reçoit des commandes JSON (Ping / OpenProject / Build / CloseProject), exécute les appels COM/UDE |
| `plcopen` | Parseur PLCopenXML (IEC 61131-3 TC6) : types Rust pour LD, FBD, ST, IL, SFC + désérialisation + renderer SVG + diff sémantique |
| `stu-vcs` | Bibliothèque VCS local pour fichiers `.stu` : store contenu-adressé SHA-256, modèle commit, diff textuel et hash |
| `cli` | Binaire `ioflow` : 7 commandes VCS (init, snapshot, log, show, diff, restore, status, config) |

---

## VCS local pour fichiers STU

`ioflow` embarque un gestionnaire de versions "git-like" pour les fichiers `.stu`
(Control Expert / Unity Pro), utilisable **sans Control Expert installé**.

### Format STU

Un fichier `.stu` est une archive ZIP contenant des fichiers de formats hétérogènes :

| Fichier | Format | Diffable ? |
|---|---|---|
| `Project_Definition.xpdf` | XML **chiffré** (Schneider) | hash uniquement |
| `Project_Settings.xso` | XML clair | diff XML complet |
| `*.db` (ASPROG, ASROOT…) | Propriétaire "eXc" (Schneider) | hash uniquement |
| `backend/gen/asm_son/*.asm` | ASM 32-bit généré | diff texte |
| `BinAppli/*.ap*` | Binaire compilé | hash uniquement |
| `*.CTX`, `*.ODB` | Propriétaires binaires | hash uniquement |

Le contenu programme (logique ladder, FBD, SFC…) est chiffré par Schneider dans le
`xpdf`. Un diff sémantique complet nécessite l'API UDE (COM/OLE). Sans UDE, `ioflow`
traque les changements par hash et propose un diff textuel sur les fichiers lisibles.

### Commandes CLI

```bash
ioflow init                              # initialise un dépôt .ioflow/
ioflow config --name "Jean Dupont"       # configure l'auteur des commits
ioflow snapshot mon_projet.stu -m "..."  # crée un snapshot (commit)
ioflow log                               # historique
ioflow show <hash>                       # détail d'un commit
ioflow diff <hash1> <hash2>             # diff entre deux commits
ioflow restore <hash> -o sortie.stu     # recrée un STU depuis un snapshot
ioflow status mon_projet.stu            # compare contre HEAD sans committer
```

Exemple de sortie `ioflow diff` :

```
Diff a3f2c1 → b7e94d
     2026-07-14 10:32 → 2026-07-14 14:05

  ~ Project_Settings.xso [XML paramètres]  1.2 KB → 1.3 KB
    --- a/Project_Settings.xso
    +++ b/Project_Settings.xso
    @@ -12,1 +12,1 @@
    -  <entryvalue ident="unity.NbWarnings" value="500">
    +  <entryvalue ident="unity.NbWarnings" value="200">
  ~ backend/gen/asm_son/code_section_001.asm [assembleur généré]  48.0 KB → 51.2 KB
    --- a/backend/gen/asm_son/code_section_001.asm
    +++ b/backend/gen/asm_son/code_section_001.asm
    @@ ... @@
    ...
  ~ ASPROG.db [base propriétaire eXc]  48.0 KB → 51.2 KB

1 modifié(s), 0 ajouté(s), 0 supprimé(s)
```

Pour les fichiers `.xso` et `.asm`, le diff textuel unifié est affiché inline (couleurs ANSI). Les binaires propriétaires (`*.db`, `*.xpdf`) n'affichent que la variation de taille.

### Modèle objet interne

```
Blob    = SHA-256(contenu brut)          → .ioflow/objects/ab/cdef…
Tree    = JSON { "fichier": blob_hash }  → stocké comme blob
Commit  = JSON { parent, tree, message, author, timestamp }
```

---

## Endpoints backend (API v1)

### Agent / jobs

| Méthode | Route | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `POST` | `/api/v1/agents/register` | Enregistrement d'un agent |
| `GET` | `/api/v1/jobs/poll` | Prochain job en file (polling agent) |
| `POST` | `/api/v1/jobs/{id}/status` | Mise à jour du statut / résultat d'un job |

### Ladder / diff (stateless — dashboard htmx)

| Méthode | Route | Description |
|---|---|---|
| `POST` | `/api/v1/render/ladder` | `{xml, pou, network}` → SVG |
| `POST` | `/api/v1/render/ladder-diff` | `{xml_a, xml_b, pou, network}` → SVG diff fusionné |
| `POST` | `/api/v1/render/ladder-diff-side` | idem → HTML deux colonnes (A rouge / B vert) |
| `POST` | `/api/v1/render/plc-semantic-diff` | `{xml_a, xml_b}` → HTML diff sémantique |

### Snapshots / diff (DB-backed)

| Méthode | Route | Description |
|---|---|---|
| `POST` | `/api/v1/snapshots` | Stocke un XML PLCopen indexé par `commit_hash` |
| `GET` | `/api/v1/snapshots/{hash}/pous` | Liste JSON des POUs |
| `GET` | `/api/v1/snapshots/{hash}/pou/{name}/ladder` | SVG d'un réseau |
| `GET` | `/api/v1/diff/{h1}/{h2}/pou/{name}/ladder` | SVG diff fusionné |
| `GET` | `/api/v1/diff/{h1}/{h2}/pou/{name}/ladder/side` | HTML deux colonnes |
| `GET` | `/api/v1/diff/{h1}/{h2}/semantic` | JSON `PlcDiff` (POUs/variables/réseaux) |

---

## Protocole IPC agent ↔ com-bridge

Commandes envoyées par l'agent sur stdin du com-bridge (JSON, une ligne par message) :

```json
{ "Ping": {} }
{ "OpenProject": { "path": "C:\\projets\\mon_projet.stu" } }
{ "Build": {} }
{ "CloseProject": {} }
```

Réponses retournées sur stdout :

```json
{ "Pong": {} }
{ "ProjectOpened": {} }
{ "BuildResult": { "success": true, "diagnostics": [], "duration_ms": 4200 } }
{ "ProjectClosed": {} }
{ "Error": { "message": "..." } }
```

---

## Schéma base de données

Tables PostgreSQL définies dans [migrations/0001_init.sql](migrations/0001_init.sql) :
`organizations`, `users`, `projects`, `agents`, `jobs`, `diagnostics`.

---

## Lancer le projet (développement)

**Prérequis :** Rust stable, PostgreSQL.

```bash
# Copier et adapter les variables d'environnement
cp .env.example .env
# Variables requises :
#   DATABASE_URL=postgres://user:pass@localhost/ioflow
#   BIND_ADDR=0.0.0.0:3000          (optionnel)

# Lancer le backend (applique les migrations automatiquement au démarrage)
cargo run -p backend

# Lancer l'agent (Windows uniquement)
# Variables requises :
#   AGENT_ID=<uuid stable>    # générer une fois : uuidgen
#   ORG_ID=<uuid org>
#   BACKEND_URL=http://localhost:3000
cargo run -p agent
```

Le `com-bridge` est compilé en cible `i686-pc-windows-msvc` :

```bash
cargo build -p com-bridge --target i686-pc-windows-msvc
```

Sans la feature `com` (par défaut), le com-bridge retourne des résultats mockés —
utile pour développer sans Control Expert installé.

---

## État d'avancement

### Infrastructure — livrée
- [x] Workspace Cargo (7 crates : shared, backend, agent, com-bridge, plcopen, stu-vcs, cli)
- [x] Schéma PostgreSQL initial
- [x] Types partagés (`Job`, `JobResult`, `Diagnostic`, protocoles HTTP/IPC)
- [x] Backend Axum + sqlx : AppState, pool PgPool, migrations auto au démarrage
- [x] Routes backend réelles : poll job (`FOR UPDATE SKIP LOCKED`), update status + diagnostics, register agent
- [x] Agent : registration au démarrage (AGENT_ID, ORG_ID), agent_id réel dans les résultats
- [x] Com-bridge : IPC JSON stdin/stdout + stubs COM/UDE
- [x] Analyse format STU (spike 2026-07-14 — voir CLAUDE.md)
- [x] CI GitHub Actions (fmt + check + clippy + tests sur Linux ; check com-bridge sur Windows)

### VCS local (`stu-vcs` + CLI) — livré
- [x] Crate `stu-vcs` (lib) — parsing STU ZIP, store SHA-256, modèle commit
- [x] `ioflow init` + `ioflow snapshot` (avec `--export` PLCopenXML optionnel)
- [x] `ioflow log` + `ioflow show`
- [x] `ioflow diff` — hash-level + diff textuel unifié ANSI pour `.xso` et `.asm`
- [x] `ioflow restore`
- [x] `ioflow status` — compare un STU local contre HEAD (calcul éphémère, store non modifié)
- [x] `ioflow config --name` — configure l'auteur dans `.ioflow/config.toml`
- [x] 25 tests d'intégration (`tests/vcs.rs`) — fixture ZIP synthétique
- [x] `rustfmt.toml` à la racine

### Parseur PLCopenXML et renderer (`plcopen`) — livré
- [x] Types complets : `Project`, `Pou`, `Interface`, `Variable`, `DataTypeRef`
- [x] Corps LD complet : contacts NO/NF, bobines SET/RESET, blocs fonctionnels
- [x] Corps ST/IL : texte brut ; FBD/SFC : stubs
- [x] Renderer SVG : `LdNetwork → SVG` avec tous les symboles IEC 61131-3 LD
- [x] Renderer diff fusionné : `render_diff` → SVG coloré (rouge/vert/jaune)
- [x] Renderer diff deux colonnes : `render_diff_columns` → `(svg_a, svg_b)`
- [x] Diff sémantique : `semantic_diff::diff_projects` → POUs/variables/réseaux
- [x] 16 tests unitaires (parser, renderer, diff, semantic_diff)

### Dashboard htmx — livré
- [x] Onglet **Afficher** — rendu SVG d'un réseau à la volée
- [x] Onglet **Diff visuel** — vue fusionnée ou côte à côte (toggle)
- [x] Onglet **Diff sémantique** — liste colorée des POUs/variables/réseaux modifiés

### Backlog
- [ ] Appels COM/UDE réels (nécessite UDE installé sur machine de test)
- [ ] Auth (sessions + argon2)
- [ ] `GET /api/v1/projects` CRUD projets
- [ ] CI PostgreSQL dans GitHub Actions (`SQLX_OFFLINE` ou DB de test)

### Inconnues techniques

- **UDE** : disponibilité et état de maintenance réels (liens cassés signalés en 2023)
- **Chiffrement xpdf** : contenu programme chiffré → diff sémantique uniquement via UDE
- **PLCopenXML** : support sur Control Expert classique (vs. Machine Expert/CODESYS)
- **Licence** : un build = une licence Control Expert sur la machine agent → modèle tarifaire à définir
- **CGU Schneider** : à valider avant tout service commercial

---

## Stack

- **Rust** (workspace Cargo unique)
- **Axum** — API backend
- **PostgreSQL + sqlx** — base de données (requêtes vérifiées à la compilation)
- **windows-rs** — appels COM/OLE vers UDE (com-bridge)
- **roxmltree** — parseur DOM pour PLCopenXML
- **sha2 / hex** — hashing SHA-256 (VCS)
- **zip** — lecture/écriture archives STU
- **similar** — diff textuel unifié (`.xso`, `.asm`)
- **clap** — CLI `ioflow`
- **htmx 2.x** — dashboard frontend sans JS framework
- **GitHub Actions** — CI (fmt + clippy + tests Linux, check Windows i686)
