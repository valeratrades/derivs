```rust
let mut tape = derivs::Tape::new(rows, stamp);
let raw = tape.source("lives_in", "pull", Some("km"), Fidelity::Exact, cells);
let v = tape.map("lives_in.v", raw, &[], Some([0.0, 80.0]), |c, _| closeness(c));
let mean = tape.weighted_mean("score", &[(v, 1.0)]);
std::fs::write("score.html", derivs_view::html(&tape))?;
```

`derivs view <tape.json>` re-renders a saved tape beside it, byte for byte. What the page reads off a tape is in [ARCHITECTURE.md](docs/ARCHITECTURE.md).
