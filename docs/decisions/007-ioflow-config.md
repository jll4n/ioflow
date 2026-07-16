# ADR 007 — Commande `ioflow config`

## Contexte

La commande `ioflow snapshot` inscrit un champ `author` dans chaque commit via
`Repo::author()`. Sans configuration explicite, cette méthode se rabat sur la
variable d'environnement `USERNAME` (Windows) ou `USER` (Unix), puis sur la chaîne
`"unknown"`. Ce comportement par défaut est acceptable en développement solo, mais
insuffisant dès qu'on partage un dépôt ou qu'on veut des commits signés clairement.

Le fichier `.ioflow/config.toml` était déjà créé par `ioflow init` avec `name = ""`
mais aucune commande ne permettait de l'écrire.

## Décision

### Interface CLI

```
ioflow config --name "Jean Dupont"   # écriture
ioflow config                        # lecture (affiche l'auteur actuel)
```

L'argument `--name` est optionnel : sans lui, la commande affiche la valeur active
(issue du fichier ou du fallback env). Ce comportement "read/write selon la présence
de l'option" suit la convention de `git config <key>` vs `git config <key> <value>`.

### Implémentation

- `Repo::set_author(name: &str)` ajouté dans `crates/stu-vcs/src/repo.rs`.
- `cmd_config(name: Option<String>)` dans `crates/cli/src/main.rs`.

### Format de `config.toml`

```toml
[user]
name = "Jean Dupont"
```

`set_author` réécrit le fichier entier. Conséquence assumée : si un utilisateur
ajoute manuellement d'autres clés (par ex. `email`), elles seraient perdues lors du
prochain `ioflow config --name`. Ce choix simplifie l'implémentation tant que le
fichier ne contient qu'une seule section avec une seule clé. Si d'autres champs
sont ajoutés à l'avenir (email, remote URL…), `set_author` devra évoluer vers une
lecture + patch partiel du TOML plutôt qu'une réécriture complète.

### Chaîne de résolution de l'auteur (inchangée)

```
1. name = "..." dans .ioflow/config.toml  (non vide)
2. Variable d'environnement USERNAME       (Windows)
3. Variable d'environnement USER           (Unix)
4. "unknown"
```

## Conséquences

- L'auteur peut désormais être configuré sans éditer manuellement `config.toml`.
- La commande `config` sans argument sert de diagnostic rapide ("qui va signer mes commits ?").
- Zéro nouvelle dépendance (pas de parser TOML — format trop simple pour le justifier).
- Si de nouvelles clés de configuration sont introduites, envisager la crate `toml`
  pour un vrai parse+update partiel.
