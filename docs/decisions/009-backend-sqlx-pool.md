# ADR 009 — Branchement sqlx + AppState backend

## Contexte

Les routes Axum étaient des stubs retournant des réponses JSON en dur. `DATABASE_URL`
était lue depuis l'environnement mais le pool n'était jamais créé. L'objectif de
cette itération est de brancher sqlx sur PostgreSQL et d'implémenter les routes
cœur du protocole agent↔backend.

## Décisions

### AppState

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}
```

`PgPool` est déjà un `Arc` interne — le `Clone` est peu coûteux. Axum clone l'état
à chaque requête via `with_state(state)`.

### Migrations au démarrage

```rust
sqlx::migrate!("../../migrations").run(&pool).await
```

Les migrations SQL (dans `migrations/`) sont appliquées automatiquement à chaque
démarrage. Idempotent grâce à la table `_sqlx_migrations` gérée par sqlx.

### Requêtes dynamiques (`sqlx::query()`) plutôt que macros (`sqlx::query!()`)

Les macros `query!` vérifient le schéma à la compilation mais nécessitent une DB
accessible (ou le mode `SQLX_OFFLINE` avec un fichier cache). Ce mode n'est pas
encore câblé en CI. On utilise donc `sqlx::query()` (vérification runtime) pour
l'instant. Migration vers `query!` + `SQLX_OFFLINE` prévue quand le job CI
PostgreSQL sera ajouté.

### `GET /api/v1/jobs/poll` — transaction `FOR UPDATE SKIP LOCKED`

```sql
SELECT id, project_id, created_at FROM jobs
WHERE status = 'queued' ORDER BY created_at LIMIT 1
FOR UPDATE SKIP LOCKED
```

La clause `FOR UPDATE SKIP LOCKED` garantit qu'en cas de polling concurrent (plusieurs
agents actifs), deux agents ne récupèrent jamais le même job. Si une ligne est déjà
verrouillée par un autre agent, elle est sautée plutôt qu'attendue. La mise à jour
(`status = 'running'`) se fait dans la même transaction.

### `POST /api/v1/jobs/{id}/status` — diagnostics en base

Les diagnostics du `JobResult` sont insérés un par un dans la table `diagnostics`.
Pas de bulk insert pour l'instant — le nombre de diagnostics par build est faible
(dizaines, pas millions).

### `POST /api/v1/agents/register` — stub en attente d'auth

L'insertion dans `agents` requiert `org_id NOT NULL` (clé étrangère vers
`organizations`). Sans contexte de session, impossible de déterminer l'organisation
de l'agent. La route logue et répond `200` sans toucher la base jusqu'à
l'implémentation de l'auth (sessions + argon2).

## Conséquences

- Le backend peut démarrer et se connecter à PostgreSQL sans configuration
  supplémentaire.
- Les deux routes critiques du protocole agent sont opérationnelles.
- La route `/register` reste un stub — à implémenter avec l'auth.
- Aucune migration vers `query!` + `SQLX_OFFLINE` avant que le job CI PostgreSQL
  soit en place.
