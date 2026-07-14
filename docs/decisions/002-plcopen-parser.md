# ADR 002 — Crate `plcopen` : parseur et types PLCopenXML

## Contexte

Le format de fichier `.stu` de Control Expert stocke le code programme dans
`Project_Definition.xpdf`, un XML **chiffré par Schneider** (Level 255), et dans
des bases de données binaires au format propriétaire **ObjectStore 7.1.0**
(eXcelon/Progress Software). Ces deux formats sont illisibles sans les outils
Schneider ou la librairie ObjectStore.

Pour proposer un diff visuel du code automate (ladder, ST, FBD…), il faut passer
par un format intermédiaire ouvert. Control Expert permet un export manuel vers
**PLCopenXML** (standard IEC 61131-3 TC6, v2.01) qui contient :
- Le code de chaque POU (Program, Function Block, Function) dans son langage
  d'origine (LD, FBD, ST, IL, SFC).
- Les déclarations de variables (interface).
- Les positions graphiques des éléments (contacts, bobines, blocs…) directement
  exploitables pour un rendu visuel sans recalcul de mise en page.

À moyen terme, l'API UDE/COM permettra de déclencher cet export automatiquement
depuis l'agent. En attendant, l'utilisateur exporte manuellement.

## Décision

Création du crate `crates/plcopen` (bibliothèque) avec trois modules :

### `types.rs` — Modèle de données

Types Rust idiomatiques couvrant le sous-ensemble PLCopenXML utile au VCS et au
rendu visuel :

```
Project
  ├── FileHeader          (métadonnées : produit, version, dates)
  ├── ContentHeader       (nom du projet, auteur, organisation)
  ├── Vec<UserDataType>   (types utilisateur — DDT/UDT, stub)
  └── Vec<Pou>
        ├── name, PouType (Program | FunctionBlock | Function)
        ├── Interface
        │     └── Vec<Variable>  par section (localVars, inputVars…)
        │           ├── name, retain, constant
        │           └── DataTypeRef  (Bool | Int | Real | Derived(…) | Array{…})
        └── Body
              ├── Ld(LdBody)   → Vec<LdNetwork> → Vec<LdElement>
              │     LdElement = LeftPowerRail | RightPowerRail
              │               | Contact  (negated, EdgeDetection, variable)
              │               | Coil     (negated, CoilStorage, CoilEdge, variable)
              │               | LdBlock  (typeName, instanceName, inputs, outputs)
              ├── Fbd(FbdBody) → stub (Vec<FbdNetwork>)
              ├── St(StBody)   → texte brut
              ├── Il(IlBody)   → texte brut
              └── Sfc(SfcBody) → XML brut (stub)
```

Chaque élément graphique porte ses coordonnées (`Position x/y`) et ses points de
connexion (`ConnectionPointIn` / `ConnectionPointOut` + `Connection { refLocalId }`),
ce qui permet un rendu SVG direct sans algorithme de placement.

Tous les types dérivent `Serialize + Deserialize` pour l'API REST (endpoint
`/api/v1/commits/{hash}/pou/{name}/ladder` → JSON → rendu SVG côté client).

### `parser.rs` — Parseur PLCopenXML

Parseur par descente récursive sur un DOM `roxmltree`. Insensible au namespace
(matching sur le nom local uniquement) pour supporter les variations d'export
entre versions de Control Expert.

Cas traités :
- Contacts NO (`negated=false`) et NF (`negated=true`)
- Détection de front (`edge`: rising / falling / both)
- Bobines SET, RESET, rétentives (`storage`)
- Blocs fonctionnels en LD (`<block typeName="TON">`) avec broches nommées
- Variables de tous types IEC 61131-3 + `derived` + `array` multidimensionnel
- Corps ST avec wrapper `<xhtml:p>` (format Control Expert)
- FBD et SFC : parsing minimal, contenu préservé pour itérations futures

### `error.rs` — Gestion d'erreurs

`ParseError` (via `thiserror`) : XML invalide, attribut manquant, type de POU
inconnu, type de donnée inconnu, POU sans corps.

## Bibliothèque retenue : `roxmltree`

| Critère | `roxmltree` | `quick-xml` (streaming) | `serde-xml-rs` |
|---|---|---|---|
| API | DOM (nœuds navigables) | Pull parser (événements) | Serde derive |
| Adéquation PLCopenXML | ✓ Idéale (structure arborescente) | Complexe (hétérogène) | Échoue sur unions discriminées |
| Gestion namespace | ✓ Transparente | Manuelle | Partielle |
| Poids | Léger (~2k lignes) | Très léger | Moyen |

PLCopenXML est un XML profondément imbriqué avec des séquences hétérogènes
(un `<network>` peut contenir `<contact>`, `<coil>`, `<block>` dans n'importe
quel ordre). Un DOM est plus lisible et moins fragile qu'un parser événementiel
pour ce cas d'usage.

## Tests

4 tests unitaires dans `parser.rs` avec une fixture XML complète représentant
un programme ladder réaliste (deux contacts, une bobine, les deux rails) :

| Test | Ce qu'il vérifie |
|---|---|
| `parse_projet_ld_basique` | Structure complète : projet → POU → interface → réseau → 5 éléments |
| `parse_variable_bool` | Type `<BOOL/>` correctement mappé |
| `parse_variable_type_derive` | Type `<derived name="TON"/>` → `DataTypeRef::Derived("TON")` |
| `connexions_correctes` | `refLocalId` correctement parsé dans `ConnectionPointIn` |

## Ce qui reste à faire (hors scope de cet ADR)

- Renderer SVG (`crates/plcopen/src/renderer/svg.rs`) : `LdNetwork` → SVG string
- Renderer diff (`renderer/diff.rs`) : deux networks → SVG avec highlights
- Parser FBD complet (éléments graphiques, wires)
- Parser SFC (étapes, transitions, actions)
- Endpoints Axum dans `crates/backend` pour exposer les POUs et leurs rendus

## Conséquences

- Le crate `plcopen` est une bibliothèque pure (pas d'I/O, pas de tokio) :
  facilement testable, réutilisable par le CLI et le backend.
- Le modèle ne couvre pas encore SFC et FBD graphique : les stubs permettront
  d'itérer sans casser l'API existante.
- Le diff sémantique complet du programme reste dépendant de l'export PLCopenXML
  (manuel ou via UDE) : sans export, seul le hash-tracking est disponible.
