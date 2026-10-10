<!--Reference: https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html-->
# Architecture

## Overview

A computation over a population — every member a row, every step a column — recorded as the DAG it is
computed through, and drawn. The caller does not describe its graph: it computes *through* a `Tape`, and
the graph is what the tape saw.

```
caller ──► Tape::{source, reduce, map, cohort, weighted_mean, gate} ──► Tape ──► read ──► Reading
                       each op is handed Ids, reads only them,                 │
                       records exactly them as its deps                        ▼
                                                     tape.json ──► derivs_view::html ──► one .html
```

- `derivs` — `Tape`, its node kinds, and `read`: what a tape says about itself, off the node kinds alone.
- `derivs_view` — a tape as one self-contained html page (the tape and its readings embedded as JSON, one
  vanilla-JS renderer), and the `derivs view <tape.json>` bin that re-renders a saved one.

## Node kinds

```
Source ──► Reduce ──► Map ──► WeightedMean ──► Gate
   │      (cohort →   (per row,  (Σw·v/Σw)     (× a multiplier,
   │       scalars:    v = f(x,                 docked on the card
   │       the edges   scalars),                it gates)
   │       across      curve = f
   │       rows)       sampled)
   └──────────────────────────────► Cohort (per row, off the whole cohort's columns)
```

## Readings

For every `WeightedMean`, and the `Gate` over it if any:

| reading | per | is |
|---|---|---|
| allocated | term | `w/Σw` |
| delivered | term | cohort mean of `w·v/Σw` |
| drives | term | `cov(w·v/Σw, mean)/var(mean)` |
| fate | term × row | `delivered + shape loss + absent = w/Σw` |
| contribution | term × row | `m·w·v/Σw`, the exact Shapley value against the all-absent row |
| gate loss | row | `mean·(1 − m)` |

Layout: `level = max(1 + level(dep), level(gate))`, as exec_viz's `dag.rs` — a gate's multiplier is drawn on
the card it gates, so it pulls that card to its own column rather than one past it.

## Invariants

- **Edges are reads**: an op reads only through the `Id`s it was given, and records exactly those as deps.
  Constants a closure captures are configuration, never data.
- **The shape plotted is the shape computed**: a `Map`'s curve is sampled from the closure that produced its
  values, at every value the cohort holds besides a uniform grid — every row's point is on its curve exactly.
- **Recording is noninvasive**: a computation through the tape yields bit-identical outputs to the computation
  without it (as `r[observe.noninvasive]` in trading_data).
- **Contributions close**: per row, Σ contributions == the gated value, and delivered − gate loss == the same.
  Asserted on every `read`, to 1e-12, never rounded away.
- **A tape is self-sufficient**: the view reads nothing but the tape; `derivs view` of a saved tape reproduces
  the page byte for byte.
- **Values are finite**: every recorded value is asserted finite at record time; JSON has no NaN.

## Not built

- panes over rank position (exec_viz's chart side, re-axed from time to the cohort sorted best-first);
- a weight-sensitivity strip (top-n stability as one weight slides);
- loops across runs (send → transcript, stale → pull order);
- polish past legibility.

The references for each are in `remote_reviews_automations/tmp/rank_vis/` (`mock/04`, `mock/05`, the Figma board).
What derivs could share with trading_data and exec_viz is in [unify.md](unify.md).
