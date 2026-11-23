# `.idef0` format 1.0

UTF-8 JSON. Generate the complete schema using `erlu schema`; generate action schemas
using `erlu actions`. Checked-in snapshots are in `docs/schema/`. Schema examples are emitted from the Rust types, not separately
maintained copies. Format versions are independent from application and SDK versions.

The envelope contains `format: "erlu"`, `format_version: {major, minor}`, `features`
with `required` and `optional` sets, `root`, `id_sequence`, `meta`, `diagrams` and
namespaced `extensions`. Root diagram ID is 1; the initial A0 block ID is 2. IDs are
globally unique positive integers below JavaScript's exact-integer limit; visual
numbers are separate. Entity maps are keyed by the decimal representation of the ID.
Coordinates are millimetres in a page-centred coordinate system, positive Y downward.
Blocks are 53 × 26.5 mm. Port `t` values are normalized to [0,1].

A diagram owns blocks, arrows and junctions; each child diagram links reciprocally
to its parent block. Endpoints contain a typed block/junction/boundary anchor and a
tunnel flag. Interface arrows reference the stable parent arrow ID; labels inherit
from that relation. Label inheritance between branch segments also uses IDs.
Graph integrity, unique IDs, acyclic hierarchy, valid endpoints and finite values
are checked on load and before committing commands.

| Feature | Introduced | Required dependency |
| --- | --- | --- |
| `idef0` | 1.0 | — |
| `idef0.decomposition` | 1.0 | `idef0` |
| `idef0.tunnels` | 1.0 | `idef0` |
| `idef0.calls` | 1.0 | `idef0` |
| `abc.costs` | 1.0 | `idef0` |
| `appearance.colors` | 1.0 | `idef0` |

The executable feature registry is `erlu-core::format::FEATURE_REGISTRY`.
A feature changing model meaning must be required. Optional features may add
ignorable metadata. Unknown required features and unknown major versions fail
before changing the active document. Newer minors may be read when every required
feature is understood. Unknown optional data survives serialization, including
nested fields in endpoints/positions; deleting an entity deliberately deletes its
associated unknown data. Replacing an anchor kind need not preserve fields specific
to the old kind. Plugin data remains opaque when its optional plugin is absent.

Future releases append pure, sequential minor migrations to `format::MIGRATIONS`.
Every released step needs a fixture and compatibility test and must remain available.
No migrations predate 1.0: the unversioned Qt format is intentionally unsupported.
Migrations run in memory; files are only rewritten on explicit save. A future minor
is not silently rewritten to an older number. Lossy downgrades are not provided.

Extension entries use `{version: <positive integer>, data: <JSON>}` under the plugin
namespace. Model-affecting extensions must declare a required capability and cannot
be silently opened by a reader lacking it. View state, history and connection tokens
are not part of the document. UI locale never changes serialized enum values.

Model metadata includes descriptive `model_abbreviation`, `purpose` and `viewpoint`
strings in addition to `title`. Older files may omit these fields. Semantic
[validation](validation.md) diagnoses missing statements without rejecting the file.
