# What derivs could share with trading_data and exec_viz

No code in trading_data or exec_viz changes for this; it maps the concepts and recommends.

| concept | where today | candidate? | why |
|---|---|---|---|
| `Fidelity` | `trading_data_dag/src/lib.rs:1763-1770`: `Exact`, `Partial(&'static str)`, `Opaque(&'static str)` | **yes** | Says nothing about ticks or lags, only how much of what a body read is covered. derivs has `Exact` and `Partial(String)`; it has no `Opaque` because every op's values are on the tape. The `hatches` census (`:1772-1789`) is worth copying either way. |
| `Want` | `lib.rs:4330-4338` | **no** | Asked per step, of the node about to fire (`Observer::want`, `:4344`), and priced per re-advance of one tick. derivs computes each node once over the whole population; there is no step to ask before. |
| `Observer` / tape | `lib.rs:4343-4356`; exec_viz `tape.rs:1-16`; rule `trading_data/docs/spec/kernels.md:39-49` | **maybe** | Opposite directions to one invariant. `r[observe.noninvasive]`: "the outs of a tick observed at `Want::Jac` MUST be identical, bit for bit, to the outs of the same tick observed at `Want::Nothing`". trading_data runs a graph once and lets an observer watch; derivs computes *through* the tape, so the graph is the record. Sharing would mean derivs recording what an `Observer` sees, not the other way round. |
| level / gate docking | exec_viz `exec_viz_web/src/dag.rs:41-66` | **yes** | Same rule, same reason: `level(node) = max(1 + level(deps), level(gates))` — "a gate … is drawn *on* this card, so it contributes its own level rather than one past it". derivs' `levels` is this without exec_viz's buffer adornment. |
| `Plot` / `Ink` | `lib.rs:822-885` | **maybe** | A node hints its slots, range and lightness/chroma; the renderer picks the hue. derivs fixes hue by a term's place in its mean. If derivs grows hints, these are the shape, and "the hue is the renderer's" stays a rule. |
| `Source` / `Provenance` | `lib.rs:1407-1439` | **no** | `Dep { dep, lag, slot }` scatters a gradient back over time-lagged reads. A population axis has no lag. |

## The seam

exec_viz keeps `trading_data_dag` out of its core with a cargo feature, `exec_viz/Cargo.toml:15-26`: `tape` (read side, plain serde types, wasm-safe) vs `server` (adds the `Observer` impl and `trading_data_dag`, native). `tape.rs` converts `Fidelity`, `Plot`, `Ink` into its own `*Out` wire types behind `cfg(feature = "server")`. A shared crate slots in at exactly that conversion.

## Recommended order

1. `Fidelity` into derivs (or a crate under both), with `Opaque` kept for trading_data — a value type, no coupling.
2. The level rule as one function both renderers call — closed over deps and gates.
3. `Plot`/`Ink`'s shape, if derivs ever lets a node hint how it is drawn.
4. Never `Want`, `Observer`, `Source`, `Provenance`: they carry tick-bound invariants derivs has no slot for.
