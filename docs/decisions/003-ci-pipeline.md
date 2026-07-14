# ADR 003 — Pipeline CI : GitHub Actions

## Contexte

Le workspace Cargo contient des crates aux contraintes très différentes :

| Crate | Contrainte plateforme | Dépendance externe |
|---|---|---|
| `shared` | Aucune | — |
| `plcopen` | Aucune | — |
| `cli` | Aucune | — |
| `backend` | Aucune | PostgreSQL (sqlx, mais sans `query!` pour l'instant) |
| `agent` | Aucune (reqwest) | — |
| `com-bridge` | **Windows uniquement** + target **i686** | windows-rs, UDE |

La feature `com` de `com-bridge` active les vrais appels COM/UDE. Elle est désactivée
par défaut (voir ADR 001). Sans cette feature, `com-bridge` se compile sur n'importe
quel runner Windows x86 sans Control Expert installé.

## Décision

Deux jobs indépendants dans `.github/workflows/ci.yml` :

### Job `ci` — Ubuntu latest

Couvre tout le workspace sauf `com-bridge`.

```
cargo fmt --all --check
cargo check --workspace --exclude com-bridge
cargo clippy --workspace --exclude com-bridge -- -D warnings
cargo test --workspace --exclude com-bridge
```

**Pourquoi Linux ?**
- Runners GitHub 2× plus rapides que Windows pour du Rust pur.
- `shared`, `plcopen`, `backend`, `agent`, `cli` n'ont aucune dépendance Windows.
- C'est là que tournent les tests unitaires de `plcopen` (les seuls tests existants).

**Clippy `-D warnings`** : tout warning Clippy est traité comme une erreur CI.
Politique assumée : le code mergé sur `main` doit être propre.

### Job `com-bridge` — Windows latest

```
cargo check -p com-bridge --target i686-pc-windows-msvc
```

Pas de tests. Les appels COM/UDE nécessitent Control Expert + UDE installés, ce
qui est impossible sur un runner GitHub public. Le check valide que le code
32 bits compile sans erreur (sans la feature `com`).

**Target `i686-pc-windows-msvc`** installée via `dtolnay/rust-toolchain@stable`
avec le paramètre `targets:`. Pas besoin de `rustup target add` séparé.

### Cache

`Swatinem/rust-cache@v2` dans les deux jobs. Cette action cache :
- Le registre Cargo (`~/.cargo/registry`)
- Le dossier `target/` (fingerprints + artefacts compilés)

Le cache est invalidé automatiquement si `Cargo.lock` change.

### Actions utilisées

| Action | Version | Rôle |
|---|---|---|
| `actions/checkout` | v4 | Récupération du code |
| `dtolnay/rust-toolchain` | `@stable` | Toolchain Rust stable + composants |
| `Swatinem/rust-cache` | v2 | Cache des dépendances Cargo |

`dtolnay/rust-toolchain` est préféré à `actions-rs/toolchain` (déprécié) et à
`rustup` brut (pas d'invalidation de cache intégrée).

## Ce qui n'est pas couvert (intentionnel)

| Sujet | Raison de l'exclusion |
|---|---|
| Tests `backend` avec PostgreSQL réel | Routes stubées, pas de `sqlx::query!` → pas nécessaire pour l'instant. À ajouter quand les requêtes seront implémentées (service `postgres` dans le job). |
| Build release `agent.exe` | Réservé au workflow `release.yml` (à créer). |
| Tests `com-bridge` réels (UDE) | Impossible sans Control Expert sur runner. Nécessiterait un self-hosted runner sur la machine de dev. |
| Tests end-to-end | Hors scope MVP. |

## Déclencheurs

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
```

Chaque PR est vérifiée avant merge. Les deux jobs sont requis pour considérer
une PR comme mergeable (à configurer dans les branch protection rules GitHub).

## Conséquences

- Tout code pushé sur `main` compile et passe Clippy sur les crates portables.
- Les tests `plcopen` (4 tests unitaires) font partie du contrat CI.
- L'ajout de nouveaux tests dans n'importe quel crate (sauf `com-bridge`) est
  automatiquement couvert sans modifier le workflow.
- Quand `backend` aura de vraies requêtes `sqlx::query!`, il faudra ajouter un
  service PostgreSQL dans le job `ci` et activer le mode offline sqlx ou une DB
  de test dédiée.
