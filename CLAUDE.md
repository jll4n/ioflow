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

## État d'avancement

- [ ] Spike : confirmer disponibilité et fonctionnement réel de UDE
- [ ] Spike : tester un export/compilation piloté par COM depuis un script simple
      (VBA ou Python) avant de porter en Rust
- [ ] Spike : PLCopenXML sur Control Expert (pas seulement Machine Expert)
- [ ] Validation marché : entretiens avec 5-10 intégrateurs/bureaux d'études
- [ ] Setup du workspace Cargo (squelette des crates)
- [ ] MVP backend : CRUD projets + jobs (sans agent réel, mock)
- [ ] MVP agent : connexion + polling (sans COM réel, mock)
- [ ] Intégration COM réelle sur une machine de test

## Questions ouvertes

- Nom du produit ?
- Modèle de pricing (par agent ? par projet ? par nombre de builds/mois ?)
- Hébergement cloud cible (VPS simple type Hetzner/OVH, ou PaaS) ?
- Faut-il un mode "on-premise complet" pour les clients réticents au cloud
  (secteur industriel souvent frileux sur la donnée) ?