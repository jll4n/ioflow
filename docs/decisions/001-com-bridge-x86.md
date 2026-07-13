# ADR 001 — COM Bridge : sous-process x86 séparé

## Contexte

L'API COM/UDE de Schneider Electric est exposée par Control Expert, un process 32 bits.
La frontière WOW64 de Windows interdit à un binaire 64 bits d'appeler directement un
serveur COM 32 bits in-process.

## Décision

- L'agent principal (`crates/agent`) est compilé en **x64** (comportement par défaut,
  aucune contrainte).
- Un second binaire (`crates/com-bridge`) est compilé en **x86** (`i686-pc-windows-msvc`)
  et gère exclusivement les appels COM/UDE vers Control Expert.
- Communication : **JSON newline-delimited sur stdin/stdout**. L'agent spawne
  `com-bridge.exe`, écrit une commande JSON par ligne, lit une réponse JSON par ligne.

## Construction

```
# Build l'agent (x64, comportement normal)
cargo build -p agent

# Build le com-bridge (x86, obligatoire pour UDE)
cargo build-bridge                # mock (feature "com" désactivée)
cargo build-bridge-com            # vrais appels COM/UDE
```

Les alias sont définis dans `.cargo/config.toml`.
La toolchain `i686-pc-windows-msvc` doit être installée : `rustup target add i686-pc-windows-msvc`.

## Feature flag `com`

Par défaut, le com-bridge répond avec des mocks (pas de dépendance `windows`).
La feature `com` active la crate `windows` et les vrais appels UDE.
Cela permet de faire tourner la CI sans Control Expert installé.

## Conséquences

- Deux binaires à livrer ensemble (`agent.exe` + `com-bridge.exe`).
- L'isolation de process protège l'agent d'un éventuel crash/panic dans la couche COM.
- La communication stdin/stdout est synchrone et simple à déboguer (loggable).
- Charge additionnelle : chaque job spawne un process com-bridge. Acceptable en MVP.
