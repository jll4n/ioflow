# ADR 010 — Enregistrement agent : org_id fourni par la config

## Contexte

La route `POST /api/v1/agents/register` était un stub : la table `agents` a une
colonne `org_id NOT NULL REFERENCES organizations(id)`, mais aucun mécanisme
d'authentification n'existe encore pour déterminer à quelle organisation appartient
un agent entrant.

Trois options envisagées :

| Option | Description | Prérequis |
|---|---|---|
| A | `org_id` dans le corps JSON (config agent) | Aucun |
| B | API token dans le header → lookup org | Table `tokens` |
| C | Session auth (argon2) | Auth complète |

## Décision — Option A

`org_id` est ajouté à `AgentRegisterRequest` dans `crates/shared/src/protocol.rs`.
L'agent le lit depuis la variable d'environnement `ORG_ID` (définie à l'installation,
comme un runner token GitHub Actions).

```
AGENT_ID=<uuid stable, généré une fois>
ORG_ID=<uuid de l'organisation>
BACKEND_URL=http://backend:3000
```

Conséquences directes :
- L'insertion en base est possible sans auth.
- L'agent est identifié de façon stable par `AGENT_ID` (UUID persistant dans le
  `.env` de la machine cliente).
- `AGENT_ID` est stable entre les redémarrages — l'UPSERT `ON CONFLICT (id) DO UPDATE`
  met à jour `hostname`, `version` et `last_seen_at` à chaque démarrage.

## Limites assumées

- Pas de vérification côté backend que `org_id` existe dans `organizations` —
  la contrainte FK PostgreSQL le garantit (l'INSERT échoue si l'org n'existe pas).
- Pas d'authentification de l'agent : n'importe qui connaissant l'URL backend
  peut enregistrer un agent avec n'importe quel `org_id`. Acceptable en MVP
  réseau local ; à durcir avec l'option B (token) avant toute exposition publique.

## Implémentation

- `shared/protocol.rs` : `org_id: Uuid` dans `AgentRegisterRequest`
- `agent/src/config.rs` : `Config::from_env()` lit `AGENT_ID` et `ORG_ID`
- `agent/src/register.rs` : appel `POST /api/v1/agents/register` au démarrage
- `agent/src/runner.rs` : `agent_id` réel passé dans `JobResult` (plus de `Uuid::nil()`)
- `backend/src/routes/agents.rs` : UPSERT `ON CONFLICT (id) DO UPDATE`
- `agent/Cargo.toml` : `reqwest` en `rustls-tls` (supprime la dépendance OpenSSL système, CI Linux compatible)

## Évolution prévue

Quand l'auth sera implémentée (option B — tokens), `org_id` sera dérivé du token
plutôt que du corps de la requête, et pourra être retiré de `AgentRegisterRequest`.
