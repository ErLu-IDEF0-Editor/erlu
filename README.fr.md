[English](README.md) · [Русский](README.ru.md) · [Français](README.fr.md) · [Español](README.es.md) · [العربية](README.ar.md) · [中文](README.zh.md)

# erlu

**Un éditeur de modèles fonctionnels IDEF0 pour ordinateur et navigateur, développé en Rust avec egui.**

erlu permet de décrire les fonctions d'un système, leurs entrées, contrôles, sorties et mécanismes. Commencez par un diagramme de contexte, puis détaillez le modèle par décomposition.

Il s'adresse aux analystes, ingénieurs et architectes qui utilisent IDEF0 pour comprendre et concevoir des processus. Les développeurs peuvent étendre l'éditeur avec des plugins WASM et connecter des outils externes et des agents d'IA.

## Fonctionnalités

- **Modélisation IDEF0 :** blocs fonctionnels, flèches orthogonales, branchements, décomposition, interfaces héritées, tunnels et références à des fonctions.
- **Historique global :** annuler/rétablir fonctionne entre diagrammes et couvre les modifications de l'interface, de l'IA et des plugins.
- **Validation :** diagnostics de structure, connexions, numérotation et règles sémantiques IDEF0.
- **Analyse des coûts :** coûts des fonctions et calcul par décomposition.
- **Export :** PDF, SVG et Markdown.
- **Présentation :** thèmes clair et sombre, couleurs des objets, interface en anglais, russe, français, espagnol, arabe et chinois simplifié.
- **Automatisation :** MCP sur ordinateur et connecteur d'agent externe dans le navigateur.

Les contrôles intégrés aident à détecter les erreurs ; ils ne constituent pas une certification complète de conformité. Voir la [portée de la validation](docs/validation.md).

## Ordinateur et web

L'application vise Linux, Windows et macOS. La version web utilise le même moteur pour le modèle, l'historique, les contrôles et l'export.

Les documents sont traités localement. Aucun compte ni serveur de stockage de documents n'est nécessaire pour éditer dans le navigateur. Ouvrez les fichiers avec le sélecteur et enregistrez-les par téléchargement. Une copie de récupération dans IndexedDB et le fonctionnement hors ligne après installation du cache sont prévus. La récupération ne remplace pas l'enregistrement des fichiers.

Linux/NixOS ainsi que Chromium, Firefox et WebKit ont été testés localement. Consultez le [bilan de validation](docs/acceptance.md) pour les autres plateformes et navigateurs.

## Plugins

Trois plugins WASM sont fournis :

| Plugin | Fonction |
| --- | --- |
| Colors | Couleurs des blocs et flèches sélectionnés |
| [Responsibility Map](plugins/responsibility_map/README.md) | Rôles organisationnels, responsabilités, contrôles et export RACI |
| [Semantic Dictionary](plugins/semantic_dictionary/README.md) | Concepts et synonymes, association aux flèches, recherche des utilisations et cohérence des libellés |

Le même composant WASM s'exécute avec Wasmtime sur ordinateur et dans un Worker dans le navigateur. Les plugins ajoutent des commandes, contrôles, exportateurs et formulaires.

Leurs modifications passent par le moteur transactionnel commun et prennent en charge annuler/rétablir. Le premier SDK ne permet pas d'enregistrer de nouvelles notations ou des widgets personnalisés arbitraires.

## Démarrage depuis les sources

### Nix / NixOS

Depuis la racine du dépôt :

```sh
nix-shell
npm --prefix web ci
npm --prefix web run plugins
```

Respectivement : lancer l'application, ouvrir l'exemple ou démarrer le web à `http://127.0.0.1:8080` :

```sh
cargo run -p erlu
```

```sh
cargo run -p erlu -- examples/order-fulfilment.idef0
```

```sh
trunk serve
```

Le shell Nix fournit les outils et bibliothèques d'exécution. Il est inutile de réinstaller Trunk avec Cargo. Un répertoire propre au compilateur sépare les artefacts Nix et rustup.

### Sans Nix

Prérequis : Rust, Node.js 22+, Python 3, Trunk et wasm-tools. La version de Rust est fixée dans `rust-toolchain.toml`.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-tools --version 1.263.0 --locked
cargo install trunk --version 0.21.14 --locked
npm --prefix web ci
npm --prefix web run plugins
```

Utilisez ensuite les commandes de lancement ci-dessus. Linux nécessite aussi les bibliothèques X11/Wayland, libxkbcommon et OpenGL.

## Commandes de base

| Action | Geste ou touche |
| --- | --- |
| Ajouter un bloc | Clic droit |
| Créer une flèche | Glisser depuis un port ou le bord du diagramme |
| Créer une branche | Ctrl + glisser depuis une flèche |
| Modifier les propriétés | Double-clic |
| Ouvrir la décomposition | Clic du milieu sur un bloc |
| Remonter d'un niveau | Clic du milieu sur le fond |
| Annuler / rétablir | Ctrl+Z / Ctrl+Shift+Z |
| Ajuster à la fenêtre | F |

Sur macOS, les raccourcis d'édition usuels utilisent Cmd.

## IA et automatisation

Connectez un client MCP à un processus indépendant, ou pilotez le document d'une fenêtre ouverte :

```sh
cargo run -p erlu -- mcp --headless
```

```sh
cargo run -p erlu -- --mcp
```

Les paramètres figurent dans le menu **AI**. Les modifications MCP partagent l'historique et les contrôles de l'interface. L'enregistrement sur disque reste explicite.

Dans le navigateur, les agents utilisent le [SDK JavaScript/TypeScript](web/sdk/erlu-agent.js). L'utilisateur autorise une origine précise et peut révoquer l'accès. L'intégration fournit l'adaptateur de l'agent ; aucun chat IA intégré n'est requis. Voir l'[API IA](docs/ai.md).

## Pour les développeurs

| Crate | Responsabilité |
| --- | --- |
| `erlu-core` | Modèle, commandes, transactions, historique, validation, géométrie et export |
| `erlu-app` | Interface egui, adaptateurs ordinateur/web et MCP |
| `erlu-plugins` | Chargement et exécution des composants WASM sur ordinateur |

L'interface, MCP, le connecteur web et les plugins partagent un registre d'actions. Les contrôles de révision détectent les conflits lors des modifications asynchrones.

Les documents utilisent un format JSON `.idef0` versionné avec une liste explicite de fonctionnalités. Un registre de migrations et la préservation des données facultatives inconnues facilitent la compatibilité future. Les anciens documents Qt et plugins Qt binaires ne sont pas pris en charge.

Le menu Fichier permet de créer des documents DFD indépendants au format `.dfd`. Consultez le [guide DFD](docs/dfd.md).

### Vérification et compilation

```sh
cargo test --workspace --locked
cargo build -p erlu --release --locked
trunk build --release --locked
npm --prefix web test
```

La version web statique se trouve dans `dist/web` et peut être hébergée en HTTPS. Le [workflow de livraison](.github/workflows/release.yml) produit des archives pour ordinateur, un ZIP web et des sommes de contrôle. Il vise GitHub Actions ; Forgejo exige une adaptation du CI. Ce workflow n'a pas encore été exécuté à distance.

## Documentation

- [Architecture](docs/architecture.md)
- [Localisation de l’interface](docs/localization.md)
- [Tests des navigateurs et CI](docs/browser-tests.md)
- [Format et compatibilité](docs/format.md)
- [API IA et MCP](docs/ai.md)
- [SDK des plugins](docs/plugins.md)
- [Validation IDEF0](docs/validation.md)
- [Bilan de validation](docs/acceptance.md)

## Licence

Le code est sous licence LGPL-3.0-or-later. Licence de la police fournie : [assets/fonts/LICENSE-DejaVu.txt](assets/fonts/LICENSE-DejaVu.txt).

[Noto Sans SC — SIL Open Font License](assets/fonts/LICENSE-NotoSansSC.txt)
