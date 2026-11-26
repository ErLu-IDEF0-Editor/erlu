# Data flow diagrams

Choose **File → New DFD** to create a logical data flow model. Open the [order fulfilment example](../examples/order-fulfilment.dfd) to see a context, a decomposition, an order store and a composite flow.

Use **External entity**, **Process** or **Data store** to place an element. To draw a flow, choose **Data flow**, then select its origin and destination and save its properties. Double-click an element or flow to edit it. Select a process and choose **Decompose** to create or open its child diagram. The left and right markers on a child diagram represent the parent process's incoming and outgoing flows. Select a marker as one end of the corresponding child flow.

Use **Data dictionary** to define atomic data items and packets made from existing definitions. Link a definition in a flow or store's properties. A boundary flow with an empty definition inherits the parent packet. To split a packet between subprocesses, add separate boundary flows and select their component definitions.

**Validation** shows structural omissions and modeling recommendations. Click a finding to open its diagram. The check compares incoming and outgoing interfaces separately and uses the union of atomic dictionary components at each inherited boundary. A component may appear on multiple child flows. The validator identifies missing and extra components, and incomplete dictionary evidence is reported separately. Use process names that describe transformations and flow names that describe the data. Review business meaning with the model's stakeholders.

The style selector switches the whole document between Gane–Sarson and Yourdon–DeMarco. Both use the same decomposition and data. The file menu exports the diagrams as SVG or PDF and the model with its dictionary as Markdown.

## File format and automation

The [v1 schema](schema/dfd-v1.schema.json) describes the `.dfd` JSON document with `format: "erlu-dfd"`. IDs are unique within that document and coordinates are in millimetres. The loader additionally checks references, diagram ownership, flow endpoints and acyclic dictionary definitions. An erlu file contains one notation. Use **Open** to switch documents.

The shared [AI interface](ai.md) supports `dfd_new` with `title` and optional `discard`. Use `dfd_edit` with an `edit` object tagged by `operation`: `node_create`, `node_update`, `flow_create`, `flow_update`, `decompose`, `style`, `definition_put` or `definition_delete`. A node endpoint is `{"kind":"node","id":2}`. A boundary endpoint is `{"kind":"boundary","parent_flow":4}`. The result of `decompose` contains the child diagram ID. Use `navigate` to open it. See `erlu actions` for command schemas and `erlu schema dfd` for the document schema.

The methodological checks follow the logical DFD conventions described in [Ed Yourdon, chapters 9 and 10](https://zimmer.fresnostate.edu/~sasanr/Teaching-Material/SAD/JESA.pdf). The Gane–Sarson shapes follow [Structured Systems Analysis](https://books.google.com/books?id=c8xWAAAAMAAJ).
