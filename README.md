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
| `backend` | API Axum : enregistrement d'agents, polling de jobs, mise à jour de statut, health check |
| `agent` | Daemon x64 : polling toutes les 5 s, orchestration du `com-bridge`, remontée des résultats |
| `com-bridge` | Binaire x86 : reçoit des commandes JSON (Ping / OpenProject / Build / CloseProject), exécute les appels COM/UDE |
| `plcopen` | Parseur PLCopenXML (IEC 61131-3 TC6) : types Rust pour LD, FBD, ST, IL, SFC + désérialisation |
| `stu-vcs` | Bibliothèque VCS local pour fichiers `.stu` : store contenu-adressé SHA-256, modèle commit, diff |
| `cli` | Binaire `ioflow` : 6 commandes VCS (init, snapshot, log, show, diff, restore) |

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
ioflow snapshot mon_projet.stu -m "..."  # crée un snapshot (commit)
ioflow log                               # historique
ioflow show <hash>                       # détail d'un commit
ioflow diff <hash1> <hash2>             # diff entre deux commits
ioflow restore <hash> -o sortie.stu     # recrée un STU depuis un snapshot
ioflow status mon_projet.stu            # compare contre HEAD
```

Exemple de sortie `ioflow diff` :

```
Snapshot a3f2c1 → b7e94d  (2026-07-14 10:32 → 2026-07-14 14:05)

Fichiers modifiés :
  ~ Project_Settings.xso
  ~ backend/gen/asm_son/code_section_001.asm
  ~ ASPROG.db  (binaire propriétaire, 48 KB → 51 KB)
  = Project_Definition.xpdf  (chiffré, hash inchangé)

--- Project_Settings.xso
+++ Project_Settings.xso
-  <entryvalue ident="unity.NbWarnings" value="500">
+  <entryvalue ident="unity.NbWarnings" value="200">
```

### Modèle objet interne

```
Blob    = SHA-256(contenu brut)          → .ioflow/objects/ab/cdef…
Tree    = JSON { "fichier": blob_hash }  → stocké comme blob
Commit  = JSON { parent, tree, message, author, timestamp }
```

---

## Endpoints backend (API v1)

| Méthode | Route | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `POST` | `/api/v1/agents/register` | Enregistrement d'un agent |
| `GET` | `/api/v1/jobs/poll` | Prochain job en file (polling agent) |
| `POST` | `/api/v1/jobs/{id}/status` | Mise à jour du statut / résultat d'un job |

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

**Prérequis :** Rust stable, PostgreSQL, `sqlx-cli`.

```bash
# Copier et adapter les variables d'environnement
cp .env.example .env

# Appliquer les migrations
sqlx migrate run

# Lancer le backend
cargo run -p backend

# Lancer l'agent (Windows uniquement)
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
- [x] Squelette backend Axum avec routes agent/jobs
- [x] Agent : boucle de polling + orchestration com-bridge
- [x] Com-bridge : IPC JSON stdin/stdout + stubs COM/UDE
- [x] Analyse format STU (spike 2026-07-14 — voir CLAUDE.md)
- [x] CI GitHub Actions (fmt + check + clippy + tests sur Linux ; check com-bridge sur Windows)

### VCS local (`stu-vcs` + CLI) — livré
- [x] Crate `stu-vcs` (lib) — parsing STU ZIP, store SHA-256, modèle commit
- [x] `ioflow init` + `ioflow snapshot` (avec `--export` PLCopenXML optionnel)
- [x] `ioflow log` + `ioflow show`
- [x] `ioflow diff` (hash-level, taille avant/après, étiquettes par type)
- [x] `ioflow restore`
- [ ] `ioflow status` — compare un STU local contre HEAD sans commit
- [ ] `ioflow config` — écrire le nom auteur dans `.ioflow/config.toml`
- [ ] Tests unitaires `stu-vcs` (fixture STU synthétique)
- [ ] Diff textuel `.xso` et `.asm` (crate `similar`)

### Parseur PLCopenXML (`plcopen`) — livré
- [x] Types complets : `Project`, `Pou`, `Interface`, `Variable`, `DataTypeRef`
- [x] Corps LD complet : contacts NO/NF, bobines SET/RESET, blocs fonctionnels
- [x] Corps ST/IL : texte brut ; FBD/SFC : stubs
- [x] 4 tests unitaires avec fixture XML
- [ ] Renderer SVG : `LdNetwork` → SVG (rendu visuel ladder)
- [ ] Renderer diff : deux networks → SVG avec highlights rouge/vert

### Backlog (post-VCS local)
- [ ] Persistance DB dans le backend (routes actuellement stubées)
- [ ] Appels COM/UDE réels (nécessite UDE installé sur machine de test)
- [ ] Dashboard web (htmx) avec rendu ladder
- [ ] Auth (sessions + argon2)

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
- **clap** — CLI `ioflow`
- **GitHub Actions** — CI (fmt + clippy + tests Linux, check Windows i686)
