# derivs
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-2026-06-29+-ab6000.svg)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/derivs-loc.json)
<br>
[<img alt="ci errors" src="https://img.shields.io/github/actions/workflow/status/valeratrades/derivs/errors.yml?branch=main&style=for-the-badge&style=flat-square&label=errors&labelColor=420d09" height="20">](https://github.com/valeratrades/derivs/actions?query=branch%3Amain) <!--NB: Won't find it if repo is private-->
[<img alt="ci warnings" src="https://img.shields.io/github/actions/workflow/status/valeratrades/derivs/warnings.yml?branch=main&style=for-the-badge&style=flat-square&label=warnings&labelColor=d16002" height="20">](https://github.com/valeratrades/derivs/actions?query=branch%3Amain) <!--NB: Won't find it if repo is private-->

A computation over a population — every member a row — recorded as the DAG it is computed through, and drawn as one self-contained html page. The graph is never described: each op reads only the ids it is handed and records exactly those as its edges.
<!-- markdownlint-disable -->
<details>
<summary>
<h2>Installation</h2>
</summary>

```toml
derivs = { git = "https://github.com/valeratrades/derivs", tag = "v0.1.0" }
derivs_view = { git = "https://github.com/valeratrades/derivs", tag = "v0.1.0" }
```

</details>
<!-- markdownlint-restore -->

## Usage
```rust
let mut tape = derivs::Tape::new(rows, stamp);
let raw = tape.source("lives_in", "pull", Some("km"), Fidelity::Exact, cells);
let v = tape.map("lives_in.v", raw, &[], Some([0.0, 80.0]), |c, _| closeness(c));
let mean = tape.weighted_mean("score", &[(v, 1.0)]);
std::fs::write("score.html", derivs_view::html(&tape))?;
```

`derivs view <tape.json>` re-renders a saved tape beside it, byte for byte. What the page reads off a tape is in [ARCHITECTURE.md](docs/ARCHITECTURE.md).



<br>

<sup>
	This repository follows <a href="https://github.com/valeratrades/.github/tree/master/best_practices">my best practices</a> and <a href="https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md">Tiger Style</a> (except "proper capitalization for acronyms": (VsrState, not VSRState) and formatting). For project's architecture, see <a href="./docs/ARCHITECTURE.md">ARCHITECTURE.md</a>.
</sup>

#### License

<sup>
	Licensed under <a href="LICENSE">Blue Oak 1.0.0</a>
</sup>

<br>

<sub>
	Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be licensed as above, without any additional terms or conditions.
</sub>

