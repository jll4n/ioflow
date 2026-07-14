# CLAUDE.md — Projet "CI/CD pour Control Expert" (nom provisoire : à définir)

Ce fichier donne le contexte du projet à Claude Code pour toute session de dev.
À garder à jour au fil de l'avancement (décisions, changements d'archi, etc.).

## Vision produit

SaaS de type "CI/CD pour projets d'automatisme Schneider Electric (Control Expert /
ex-Unity Pro)", ciblant en priorité les intégrateurs et bureaux d'études automatisme.

Problème adressé : les projets automates (programmes PLC pour Modicon M340/M580/
Quantum/Premium) sont aujourd'hui versionnés "à la main" (copies de fichiers, noms
type `projet_v3_final_DEF.stu`), sans diff lisible, sans compilation automatique à
chaque modification, sans tests automatisés, sans traçabilité claire des changements
avant un déploiement sur site.

Objectif MVP : permettre à un bureau d'études de connecter un dépôt Git contenant
l'export d'un projet Control Expert, et de déclencher automatiquement à chaque
commit : une compilation, un rapport d'erreurs/warnings, et un diff humainement
lisible entre deux versions.

## Contexte utilisateur (moi)

- Fond métier : électronique / automatisme / électricité industrielle.
- Reconversion en cours vers l'IT, avec appétence pour le bas niveau (systèmes,
  réseaux, performance).
- Objectif secondaire du projet : construire un portfolio technique solide en Rust
  bas niveau (FFI/COM Windows, protocoles, orchestration) en plus de viser un vrai
  produit commercialisable.
- Niveau Rust : à préciser au fil des sessions (indiquer ici la progression réelle
  pour que Claude adapte le niveau d'explication : débutant / intermédiaire / avancé).

## Contraintes techniques clés (à valider en priorité — non bloquantes pour commencer)

1. **UDE (Unity Developer's Edition)** : extension Schneider exposant une API
   COM/OLE pour piloter Control Expert (ouverture projet, compilation, extraction).
   Disponibilité et statut de maintenance à reconfirmer directement auprès de
   Schneider/un revendeur (des utilisateurs ont signalé des liens de téléchargement
   cassés en 2023).
2. **Licence Control Expert** : nécessaire sur chaque machine qui exécute des builds.
   Impossible de mutualiser côté cloud → implique un modèle "agent auto-hébergé"
   plutôt qu'un SaaS 100% cloud (voir architecture ci-dessous).
3. **Format PLCopenXML** : standard d'export/import de POU/FB/FC, bien documenté
   sur EcoStruxure Machine Expert (branche CODESYS de Schneider) ; support réel sur
   Control Expert classique (Unity Pro) à vérifier — pourrait ne pas être 1:1.
4. **CGU Schneider** : à relire avant de construire un service commercial reposant
   sur l'automatisation de leur logiciel.

Ces points sont des inconnues techniques assumées : le projet démarre en mode
recherche/spike sur ces 4 sujets avant tout développement produit sérieux.

## Stack technique (choix provisoires)

- **Langage principal** : Rust (workspace Cargo unique, plusieurs crates).
- **Backend API** : Axum (async, écosystème mature, bon fit Rust idiomatique).
- **Base de données** : PostgreSQL + `sqlx` (requêtes vérifiées à la compilation).
- **Agent local (Windows)** : Rust + crate `windows-rs` pour les appels COM/OLE
  vers UDE. Tourne comme un service Windows ou un simple process longue durée.
- **Communication agent ↔ backend** : HTTP(S) + websockets (ou long-polling en
  MVP) — l'agent est un "runner" qui vient chercher du travail (pull), pas
  l'inverse, pour éviter d'exposer les machines clientes sur Internet.
- **Frontend dashboard** : à trancher (options : Leptos/Yew en pur Rust+WASM pour
  rester full-Rust, ou un simple frontend HTML/htmx servi par Axum pour aller vite
  en MVP). Recommandation actuelle : htmx pour le MVP, migration Leptos possible
  ensuite si besoin d'interactivité riche.
- **Auth** : à définir (probablement sessions + argon2 pour le MVP, OAuth plus tard).
- **CI/CD du projet lui-même** : GitHub Actions.

## Architecture provisoire (vue d'ensemble)

```
┌─────────────────────────────┐
│   Client (bureau d'études)  │
│                              │
│  ┌────────────────────────┐ │
│  │ Poste Windows           │ │
│  │ - Control Expert + UDE  │ │
│  │ - Agent Rust (runner)   │◄├──── pull jobs (HTTPS) ────┐
│  │ - Repo Git local        │ │                            │
│  └────────────────────────┘ │                            │
└─────────────────────────────┘                            │
                                                             ▼
                                          ┌──────────────────────────────┐
                                          │   Backend cloud (Rust/Axum)  │
                                          │  - API REST/WS                │
                                          │  - File d'attente de jobs     │
                                          │  - Stockage rapports/diffs    │
                                          │  - Auth / multi-tenant        │
                                          └──────────────┬────────────────┘
                                                          │
                                          ┌───────────────▼────────────────┐
                                          │  PostgreSQL                    │
                                          │  - projets, jobs, runs,        │
                                          │    rapports, users, orgs       │
                                          └─────────────────────────────────┘
                                                          ▲
                                                          │
                                          ┌───────────────┴────────────────┐
                                          │  Dashboard web (htmx/Leptos)   │
                                          │  - historique des builds       │
                                          │  - diffs lisibles              │
                                          │  - statut des agents           │
                                          └─────────────────────────────────┘
```

Principe clé : le backend cloud n'a jamais besoin d'avoir Control Expert installé.
Il orchestre uniquement des jobs que des agents (chez les clients, sur des machines
sous licence) viennent récupérer et exécuter, un peu comme les "self-hosted
runners" de GitHub Actions ou GitLab CI.

## Flux MVP (premier cas d'usage à livrer)

1. L'utilisateur connecte un dépôt Git (ou upload un export de projet) via le
   dashboard.
2. Il installe l'agent Rust sur son poste Windows (avec Control Expert + UDE).
3. À chaque nouveau commit détecté (ou déclenchement manuel), le backend crée un
   job "build" en base.
4. L'agent, en polling, récupère le job, ouvre le projet dans Control Expert via
   COM, lance la compilation.
5. L'agent remonte au backend : succès/échec, liste des erreurs/warnings, durée.
6. Le dashboard affiche l'historique et, si possible, un diff structurel entre la
   version courante et la précédente.

## Arborescence provisoire du repo

```
saas-control-expert/
├── CLAUDE.md                     # ce fichier
├── Cargo.toml                    # workspace root
├── Cargo.lock
├── .github/
│   └── workflows/
│       ├── ci.yml                # tests + clippy + fmt sur chaque PR
│       └── release.yml           # build agent Windows + déploiement backend
│
├── crates/
│   ├── shared/                   # types partagés agent <-> backend (protocole)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── job.rs             # struct Job, JobStatus, JobResult...
│   │       └── protocol.rs        # (de)sérialisation des messages agent<->API
│   │
│   ├── backend/                  # API cloud (Axum)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── projects.rs
│   │       │   ├── jobs.rs
│   │       │   ├── agents.rs
│   │       │   └── auth.rs
│   │       ├── db/
│   │       │   ├── mod.rs
│   │       │   └── models.rs
│   │       ├── services/          # logique métier (queue de jobs, diff, etc.)
│   │       └── config.rs
│   │
│   ├── agent/                    # runner Windows local
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── com/               # wrapper autour de l'API COM/UDE
│   │       │   ├── mod.rs
│   │       │   └── control_expert.rs
│   │       ├── poller.rs          # récupération des jobs depuis le backend
│   │       ├── runner.rs          # exécution d'un job (build, extraction...)
│   │       └── git.rs             # interactions avec le repo local
│   │
│   └── cli/                      # (optionnel) outil en ligne de commande
│       ├── Cargo.toml
│       └── src/main.rs
│
├── migrations/                   # migrations sqlx (SQL brut versionné)
│   └── 0001_init.sql
│
├── web/                          # frontend (dashboard)
│   ├── templates/                # si htmx + Askama
│   └── static/
│       ├── css/
│       └── js/
│
├── docs/
│   ├── architecture.md
│   ├── decisions/                # ADR (Architecture Decision Records)
│   └── recherche-technique.md    # notes sur UDE, PLCopenXML, licences...
│
└── scripts/
    └── dev-setup.sh
```

## Conventions de travail avec Claude

- Toujours vérifier l'état réel du code avant de proposer un diff (ne pas halluciner
  l'existant).
- Préférer des étapes petites et testables : chaque session doit se conclure sur
  quelque chose qui compile / passe les tests.
- Documenter les décisions d'architecture importantes dans `docs/decisions/` au fur
  et à mesure (format ADR court : contexte / décision / conséquences).
- Ne pas ajouter de dépendance lourde sans la justifier ici.
- Les inconnues techniques listées plus haut (UDE, PLCopenXML, licences) doivent
  être levées via des spikes isolés, documentés dans
  `docs/recherche-technique.md`, avant d'être considérées comme acquises dans
  l'architecture.

## Module prioritaire : stu-vcs (gestionnaire de versions STU)

Avant toute intégration COM/cloud, le chantier actif est un outil CLI "git-like"
pour versionner les fichiers `.stu` localement, sans dépendance à Control Expert.

### Analyse du format STU (résultats du spike du 2026-07-14)

Un fichier `.stu` est une **archive ZIP** contenant :

| Fichier / dossier | Format | Lisible ? | Intérêt pour le diff |
|---|---|---|---|
| `Project_Definition.xpdf` | XML **chiffré** (Schneider Level 255) | ✗ | Hash uniquement |
| `Project_Settings.xso` | XML clair | ✓ | Diff XML complet |
| `*.db` (ASPROG, ASROOT, VariableManager…) | Format propriétaire "eXc" (magic `eXc\r\n`) | ✗ | Hash uniquement |
| `backend/gen/asm_son/*.asm` | ASM 32-bit généré | ✓ | Diff texte (révèle sections modifiées) |
| `BinAppli/*.ap{b,d,x}` | Binaire compilé | ✗ | Hash uniquement |
| `*.CTX`, `*.ODB` | Formats propriétaires binaires | ✗ | Hash uniquement |
| `IOS/*.bmp` | Bitmap | ✗ | Hash uniquement |

Conséquences directes :
- Le contenu programme (logique ladder, FBD, SFC…) est **chiffré côté Schneider**
  → on ne peut pas proposer de diff sémantique sans passer par l'API UDE.
- On peut néanmoins tracker **quels fichiers ont changé** entre deux versions,
  proposer un diff textuel sur les fichiers lisibles, et stocker les snapshots
  de façon déduplicatée (contenu-adressé par SHA-256).
- Les fichiers `.asm` générés révèlent des noms de sections et leur contenu
  assembleur → indice partiel des blocs modifiés même sans déchiffrement.

### Architecture du VCS (nouveau crate `crates/stu-vcs`)

**Modèle objet (inspiré de Git, simplifié) :**

```
Blob    = SHA-256(contenu brut du fichier)
Tree    = { "nom_fichier": blob_hash, ... }  → sérialisé en JSON, stocké comme blob
Commit  = { parent: Option<Hash>, tree: Hash, message: String,
            author: String, timestamp: DateTime<Utc> }
```

**Layout du dépôt local :**

```
.ioflow/
  HEAD                   # "ref: refs/heads/main" ou hash direct
  config.toml            # [repo] name ; [user] name, email
  refs/
    heads/
      main               # hash SHA-256 du dernier commit
  objects/               # store contenu-adressé
    ab/
      cdef1234…          # contenu brut (blob, tree JSON, commit JSON)
```

**Stratégie de diff par type de fichier :**

| Fichier | Stratégie |
|---|---|
| `*.xso` | Diff XML structurel (element/attribut) |
| `*.asm` | Diff texte ligne à ligne |
| `*.xpdf` (chiffré) | "contenu chiffré — modifié : oui/non" |
| `*.db`, `*.CTX`, `*.ODB` | "binaire propriétaire — modifié : oui/non, Δtaille" |
| `*.apb/apd/apx`, `*.bmp` | "binaire — modifié : oui/non, Δtaille" |

**Commandes CLI cibles (`ioflow` dans `crates/cli`) :**

```
ioflow init [--path <dir>]                    # initialise un dépôt dans .ioflow/
ioflow snapshot <fichier.stu> [-m "message"]  # crée un commit à partir d'un STU
ioflow log                                    # historique des commits
ioflow show <hash>                            # détail d'un commit (fichiers modifiés)
ioflow diff <hash1> <hash2>                   # diff entre deux commits
ioflow restore <hash> -o <sortie.stu>         # recrée un STU depuis un snapshot
ioflow status <fichier.stu>                   # compare un STU contre HEAD
```

**Modules internes du crate `stu-vcs` :**

```
crates/stu-vcs/
  src/
    lib.rs
    stu.rs          # parsing STU (extraction ZIP, inventaire des fichiers)
    objects.rs      # store contenu-adressé (SHA-256, lecture/écriture blobs)
    tree.rs         # objet Tree (serialisation JSON du snapshot de fichiers)
    commit.rs       # objet Commit (parent, tree, métadonnées)
    repo.rs         # gestion du dépôt (init, HEAD, refs)
    diff.rs         # moteur de diff inter-commits (dispatch par type)
    xml_diff.rs     # diff XML pour *.xso
```

### Arborescence réelle

```
ioflow/
├── crates/
│   ├── shared/       # types Job/Protocol partagés agent↔backend
│   ├── backend/      # API Axum cloud (routes stubées)
│   ├── agent/        # daemon x64 polling
│   ├── com-bridge/   # sous-processus x86 COM/UDE (mocks)
│   ├── plcopen/      # parseur PLCopenXML (LD, ST, IL, stubs FBD/SFC)
│   ├── stu-vcs/      # VCS local : store SHA-256, Tree, Commit, diff
│   └── cli/          # binaire ioflow (init, snapshot, log, show, diff, restore)
├── stuexample/       # STU dézippé — référence reverse engineering format
├── docs/
│   └── decisions/    # ADR 001 à 004
├── .github/
│   └── workflows/
│       └── ci.yml    # CI : fmt + check + clippy + tests (Linux + Windows i686)
├── migrations/       # SQL sqlx
├── TODO.md           # backlog priorisé
└── CLAUDE.md         # ce fichier
```

## État d'avancement

### Infrastructure existante
- [x] Workspace Cargo (5 crates : shared, backend, agent, com-bridge, cli)
- [x] Schéma PostgreSQL initial
- [x] Types partagés (Job, JobResult, Diagnostic, protocoles HTTP/IPC)
- [x] Squelette backend Axum avec routes agent/jobs
- [x] Agent : boucle de polling + orchestration com-bridge
- [x] Com-bridge : IPC JSON stdin/stdout + stubs COM/UDE

### Livré

- [x] Workspace Cargo (7 crates)
- [x] Schéma PostgreSQL initial + types partagés
- [x] Backend Axum squelette (routes stubées)
- [x] Agent polling + com-bridge IPC JSON (mocks)
- [x] CI GitHub Actions (2 jobs : Linux + Windows i686)
- [x] Crate `plcopen` : parseur PLCopenXML complet pour LD + ST/IL, stubs FBD/SFC
- [x] Crate `stu-vcs` : store SHA-256, Tree, Commit, diff par type de fichier
- [x] CLI `ioflow` : init, snapshot, log, show, diff, restore
- [x] ADR 001 à 004 dans `docs/decisions/`
- [x] `TODO.md` à la racine

### En cours / prochaine itération

- [ ] `ioflow status` — comparer STU local contre HEAD sans commit
- [ ] `ioflow config` — écrire le nom auteur
- [ ] Tests unitaires `stu-vcs` (fixture STU synthétique)
- [ ] Diff textuel `.xso` et `.asm` (crate `similar`)
- [ ] `rustfmt.toml` à la racine (éviter les allers-retours CI fmt)

### Backlog (post-VCS local)
- [ ] Persistance DB dans le backend (routes actuellement stubées)
- [ ] Appels COM/UDE réels (nécessite UDE sur machine de test)
- [ ] Renderer SVG ladder (`plcopen`)
- [ ] Dashboard web (htmx)
- [ ] Auth (sessions + argon2)
- [ ] Spike : PLCopenXML sur Control Expert (vs. Machine Expert)
- [ ] Validation marché : entretiens avec 5-10 intégrateurs/bureaux d'études

## Questions ouvertes

- Nom du produit ?
- Modèle de pricing (par agent ? par projet ? par nombre de builds/mois ?)
- Hébergement cloud cible (VPS simple type Hetzner/OVH, ou PaaS) ?
- Faut-il un mode "on-premise complet" pour les clients réticents au cloud
  (secteur industriel souvent frileux sur la donnée) ?
- Le chiffrement Schneider du `xpdf` est-il contournable légalement/techniquement
  pour proposer un diff sémantique sans UDE ? (probablement non → dépendance UDE assumée)