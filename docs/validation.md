# IDEF0 validation and ISO/IEC/IEEE 31320-1:2012

The semantic validator uses the international IDEF0 publication as its reference.
It does not use FIPS PUB 183 as the normative basis. National or company rules
must not silently replace the international rules.

## Evidence and limits

The [ISO catalogue](https://www.iso.org/standard/60615.html) identifies the current
publication. The [IEEE catalogue](https://standards.ieee.org/ieee/31320-1/5545/)
identifies it as an adoption of IEEE 1320.1-1998. The international foreword in the
[ANSI preview](https://webstore.ansi.org/preview-pages/ISO/preview_ISO%2BIEC%2BIEEE%2B31320-1-2012.pdf)
confirms that relationship.

The implementation was checked against these public excerpts of the primary text:

- [International preview, clause 2.1.1](https://cdn.standards.iteh.ai/samples/60615/1a1f86fa91544e25b8ae53954ba73688/ISO-IEC-IEEE-31320-1-2012.pdf):
  the required A-0 context and its model identification, purpose and viewpoint.
- [International preview, printed pages 48-58](https://www.normsplash.com/Samples/ISO/162743767/ISO-IEC-IEEE-31320-1-2012-en-2.pdf):
  Annex B definitions 12-22 and the explanatory material in Annex C.

**Coverage is partial.** The complete normative body was not available during
this change. Annex B is explicitly informative; its formalization is evidence of
the published model structure, not an independently binding set of requirements.
Diagnostics based on it explicitly say `informative formalization`. These checks
implement that structure, but must not be described as a complete audit of every
mandatory provision. The normative clauses and references must be audited against
a complete authorized copy before making a full conformance claim.

## Implemented checks

| Code | Check | Evidence |
| --- | --- | --- |
| `model_name` | Nonblank full model name | 2.1.1 |
| `model_abbreviation` | Nonblank model name abbreviation | 2.1.1 |
| `model_purpose` | Nonblank purpose statement | 2.1.1 |
| `model_viewpoint` | Nonblank viewpoint statement | 2.1.1 |
| `context_block_count` | Exactly one context box, including detection of an empty context | 2.1.1; B.2.2, definition 13 |
| `decomposition_block_count` | 2-9 boxes on decomposition diagrams | B.2.2, definition 14 |
| `node_numbering` | Context node A0; unique, contiguous child numbers obtained by appending one digit to the parent, with A1-A9 below A0 | B.2.1-2, definitions 12-14 |
| `missing_control` | Each function has a control; input alone is insufficient | B.2.1, definition 12 |
| `missing_output` | Each function has an output | B.2.1, definition 12 |
| `incomplete_interface` | Existing, untunneled interface placeholders must be connected | B.3, definition 20; partial interface check |
| `missing_boundary_interface` | Untunneled child boundaries reference a parent interface | B.3, definition 20 |
| `invalid_parent_interface` | The referenced arrow touches this diagram’s parent box in the corresponding incoming/outgoing direction | B.3, definition 20 |
| `boundary_direction` | Incoming boundaries use I/C/M; outgoing boundaries use O | Incoming/outgoing correspondence in B.3, definition 20 |

Inputs and non-call mechanisms are optional. A connected tunneled control still
counts as a control. Interface placeholders and call arrows do not satisfy the
control/output checks. The 2-9 range differs from the historical FIPS 3-6 rule and
from the former editor-specific 3-8 range.

`source` and `reference` accompany diagnostics in the API and GUI. References name
the excerpt actually inspected; they do not pretend that the inaccessible normative
clauses were reviewed. Stable code `missing_input_control` was replaced by
`missing_control` because its former meaning allowed input to substitute for control.

## Editor advisories

Matching input/output labels and repeated labels are warnings with `source: erlu`;
they do not prove that object meanings coincide. Cost aggregation, all-tunneled
connections and absence of a direct sibling connection remain editor warnings.
An empty block title is an editor error; an unchanged placeholder title is an editor warning. Grammatical correctness and whether a
name meaningfully describes a function require human review.

The former `interface_role_change` and `output_as_mechanism` warnings were removed:
role changes are permitted by Annex B definition 20 and an output can supply a
resource to another activity. Neither case is automatically an error.

## Representation and compatibility

`meta.title` is the full model name. `meta.model_abbreviation`, `meta.purpose` and
`meta.viewpoint` are descriptive strings editable in Properties and through
`document_metadata`. Missing fields in older v1 files default to empty strings;
those files remain editable and receive missing-statement diagnostics. No file
migration or automatic save is performed.

New node numbers concatenate digits (A11, A121), replacing the former dotted
presentation (A1.1). Existing dotted values are preserved on load and diagnosed;
renumbering a parent or its children generates canonical numbers. Deleting a child
box compacts the surviving sibling numbers while preserving their order and stable
IDs, and updates descendant numbers in the same undoable transaction. Deleting
an arrow does not renumber unrelated nodes. The root diagram reference is A-0;
this is distinct from its box node A0.

The validator does not prevent editing, loading, saving or exporting an unfinished
model; structural integrity remains a separate prerequisite for engine operations.
A clean report covers only the checks listed above. It does not certify page layout,
context-statement presentation in SVG/PDF, grammar, arrow-label meaning, all tunnel
relations, call semantics, ICOM numbering, optional context diagrams or all clauses
of the standard. Those areas need the complete normative text and, where necessary,
additional model or rendering support.
