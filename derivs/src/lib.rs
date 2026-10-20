#![feature(default_field_values)]
//! A computation over a population, recorded as the DAG it is computed through. See `docs/ARCHITECTURE.md`.

mod read;

pub use read::{Factor, Fate, Reading, Term, levels, read};
use serde::{Deserialize, Serialize};

/// Uniform samples a `Map`'s curve takes over its domain, besides every value the cohort holds.
const SAMPLES: usize = 256;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Tape {
	/// The population axis: one row per member, in the order every per-row column is.
	pub rows: Vec<String>,
	/// What produced the tape, for reproducing it: a config hash, a binary version.
	pub stamp: String,
	nodes: Vec<Node>,
}
impl Tape {
	pub fn new(rows: Vec<String>, stamp: String) -> Self {
		Self { rows, stamp, nodes: Vec::new() }
	}

	pub fn nodes(&self) -> &[Node] {
		&self.nodes
	}

	pub fn node(&self, id: Id) -> &Node {
		&self.nodes[id.0 as usize]
	}

	/// The per-row values of `id`, as plain numbers.
	pub fn at(&self, id: Id) -> Vec<Option<f64>> {
		self.rows(id)
			.iter()
			.map(|cell| {
				cell.as_ref().map(|cell| match cell {
					Cell::At(v) => *v,
					cell => panic!("`{}` holds {cell:?}, and only a source's cell is anything but a point", self.node(id).name),
				})
			})
			.collect()
	}

	pub fn source(&mut self, name: &str, writer: &str, unit: Option<&str>, fidelity: Fidelity, cells: Vec<Option<Cell>>) -> Id {
		assert_eq!(cells.len(), self.rows.len(), "`{name}`: one cell per row");
		for cell in cells.iter().flatten() {
			let finite = match cell {
				Cell::At(v) => v.is_finite(),
				Cell::Within(lo, hi) => lo.is_finite() && hi.is_finite() && lo < hi,
				Cell::Each(items) => !items.is_empty() && items.iter().all(|v| v.is_finite()),
			};
			assert!(finite, "`{name}` was handed {cell:?}");
		}
		let kind = Kind::Source {
			writer: writer.to_string(),
			unit: unit.map(str::to_string),
		};
		self.push(name, kind, &[], Vals::Rows(cells), Some(fidelity))
	}

	pub fn reduce<const N: usize>(&mut self, name: &str, deps: &[Id], names: [&str; N], f: impl FnOnce(&[&[Option<Cell>]]) -> Option<[f64; N]>) -> Id {
		let scalars = f(&self.columns(deps));
		if let Some(scalars) = &scalars {
			assert!(scalars.iter().all(|v| v.is_finite()), "`{name}` reduced to {scalars:?}");
		}
		let kind = Kind::Reduce {
			names: names.iter().map(|n| n.to_string()).collect(),
		};
		self.push(name, kind, deps, Vals::Scalars(scalars.map(Vec::from)), None)
	}

	/// `domain` is the x the curve is drawn over, `None` taking the cohort's extent.
	pub fn map(&mut self, name: &str, x: Id, scalars: &[Id], domain: Option<[f64; 2]>, f: impl Fn(&Cell, &[f64]) -> f64) -> Id {
		if let Some([lo, hi]) = domain {
			assert!(lo.is_finite() && hi.is_finite() && lo < hi, "`{name}`: a domain is a span, and is [{lo}, {hi}]");
		}
		let mut deps = vec![x];
		let mut given = Some(Vec::new());
		for id in scalars {
			let Vals::Scalars(s) = &self.node(*id).vals else {
				panic!("`{name}` reads `{}` as scalars, and it is per row", self.node(*id).name)
			};
			given = given.zip(s.as_ref()).map(|(mut all, s)| {
				all.extend(s);
				all
			});
			deps.push(*id);
		}
		let xs = self.rows(x);
		let Some(given) = given else {
			assert!(xs.iter().all(Option::is_none), "`{name}`: a reduction over the cohort came out empty while a row holds a value");
			return self.push(name, Kind::Map { curve: None, domain }, &deps, Vals::Rows(vec![None; xs.len()]), None);
		};
		let eval = |cell: &Cell| {
			let v = f(cell, &given);
			assert!(v.is_finite(), "`{name}` of {cell:?} came out {v}");
			v
		};
		let vals: Vec<Option<Cell>> = xs.iter().map(|cell| cell.as_ref().map(|cell| Cell::At(eval(cell)))).collect();
		let mut points: Vec<f64> = xs
			.iter()
			.flatten()
			.flat_map(|cell| match cell {
				Cell::At(v) => vec![*v],
				Cell::Within(lo, hi) => vec![*lo, *hi],
				Cell::Each(items) => items.clone(),
			})
			.collect();
		let span = domain.or_else(|| (!points.is_empty()).then(|| points.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], v| [lo.min(*v), hi.max(*v)])));
		let curve = span.map(|[lo, hi]| {
			points.extend((0..=SAMPLES).map(|i| lo + (hi - lo) * i as f64 / SAMPLES as f64));
			points.sort_by(f64::total_cmp);
			points.dedup();
			points.into_iter().map(|t| [t, eval(&Cell::At(t))]).collect()
		});
		self.push(name, Kind::Map { curve, domain }, &deps, Vals::Rows(vals), None)
	}

	pub fn cohort(&mut self, name: &str, deps: &[Id], f: impl FnOnce(&[&[Option<Cell>]]) -> Vec<Option<f64>>) -> Id {
		let vals = f(&self.columns(deps));
		assert_eq!(vals.len(), self.rows.len(), "`{name}`: one value per row");
		assert!(vals.iter().flatten().all(|v| v.is_finite()), "`{name}` came out {vals:?}");
		self.push(name, Kind::Cohort, deps, Vals::Rows(vals.into_iter().map(|v| v.map(Cell::At)).collect()), None)
	}

	pub fn weighted_mean(&mut self, name: &str, terms: &[(Id, f64)]) -> Id {
		assert!(terms.iter().all(|(_, w)| w.is_finite() && *w > 0.0), "`{name}`: a weight is a positive share");
		let total: f64 = terms.iter().map(|(_, w)| w).sum();
		let columns: Vec<Vec<Option<f64>>> = terms.iter().map(|(id, _)| self.at(*id)).collect();
		let vals = (0..self.rows.len())
			.map(|i| Some(Cell::At(terms.iter().zip(&columns).map(|((_, w), column)| w * column[i].unwrap_or(0.0)).sum::<f64>() / total)))
			.collect();
		let kind = Kind::WeightedMean {
			weights: terms.iter().map(|(_, w)| *w).collect(),
		};
		let deps: Vec<Id> = terms.iter().map(|(id, _)| *id).collect();
		self.push(name, kind, &deps, Vals::Rows(vals), None)
	}

	/// `of` × the product of `by`, an absent factor being none.
	pub fn gate(&mut self, name: &str, of: Id, by: &[Id]) -> Id {
		assert!(!by.is_empty(), "`{name}`: a gate is gated by something");
		let multiplier = self.multiplier(by);
		let vals = self
			.at(of)
			.iter()
			.zip(multiplier)
			.map(|(v, m)| Some(Cell::At(v.expect("a gated node holds a value on every row") * m)))
			.collect();
		let deps: Vec<Id> = std::iter::once(of).chain(by.iter().copied()).collect();
		self.push(name, Kind::Gate, &deps, Vals::Rows(vals), None)
	}

	/// Per row, the product of `factors`, an absent one being 1.
	fn multiplier(&self, factors: &[Id]) -> Vec<f64> {
		let columns: Vec<Vec<Option<f64>>> = factors.iter().map(|id| self.at(*id)).collect();
		(0..self.rows.len()).map(|i| columns.iter().map(|c| c[i].unwrap_or(1.0)).product()).collect()
	}

	fn rows(&self, id: Id) -> &[Option<Cell>] {
		match &self.node(id).vals {
			Vals::Rows(rows) => rows,
			Vals::Scalars(_) => panic!("`{}` is scalars, read per row", self.node(id).name),
		}
	}

	fn columns(&self, deps: &[Id]) -> Vec<&[Option<Cell>]> {
		deps.iter().map(|id| self.rows(*id)).collect()
	}

	/// `fidelity` is a source's own; anything else is as partial as what it reads.
	fn push(&mut self, name: &str, kind: Kind, deps: &[Id], vals: Vals, fidelity: Option<Fidelity>) -> Id {
		assert!(self.nodes.iter().all(|n| n.name != name), "`{name}` is recorded twice");
		let fidelity = fidelity.unwrap_or_else(|| match deps.iter().map(|id| self.node(*id)).find(|n| n.fidelity != Fidelity::Exact) {
			Some(partial) => Fidelity::Partial(format!("reads `{}`", partial.name)),
			None => Fidelity::Exact,
		});
		self.nodes.push(Node {
			name: name.to_string(),
			kind,
			deps: deps.to_vec(),
			vals,
			fidelity,
		});
		Id(u32::try_from(self.nodes.len() - 1).expect("fewer than 2^32 nodes"))
	}
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Id(u32);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Node {
	pub name: String,
	pub kind: Kind,
	pub deps: Vec<Id>,
	pub vals: Vals,
	pub fidelity: Fidelity,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Kind {
	/// What a row brings in from outside. `writer` is whoever refreshes it.
	Source { writer: String, unit: Option<String> },
	/// The whole cohort into named scalars: the edges that run across rows.
	Reduce { names: Vec<String> },
	/// Per row, `v = f(x, scalars)` of its first dep, the others being `Reduce`s. `curve` is `f` sampled over `domain` and at every value the cohort holds, `None` when there is neither to put one under.
	Map { curve: Option<Vec<[f64; 2]>>, domain: Option<[f64; 2]> },
	/// Per row, out of the whole cohort's columns.
	Cohort,
	/// `Σ w·v / Σ w`, an absent `v` crediting 0.
	WeightedMean { weights: Vec<f64> },
	/// Its first dep × the product of the rest, an absent factor being none.
	Gate,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Vals {
	Rows(Vec<Option<Cell>>),
	/// `None` when the cohort held nothing to reduce.
	Scalars(Option<Vec<f64>>),
}

/// One row's value. Everything computed is `At`; only a source may know a value to a range, or as many items.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Cell {
	At(f64),
	Within(f64, f64),
	Each(Vec<f64>),
}

/// Same names and meaning as `trading_data_dag::Fidelity`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Fidelity {
	Exact,
	Partial(String),
}

#[cfg(test)]
mod tests {
	use super::*;

	/// rows a, b, c: `x` = 1, absent, 3; `max` over it; `v = x / max`; a mean of `v` and `y`, gated by `m`.
	fn tape() -> (Tape, [Id; 6]) {
		let mut tape = Tape::new(vec!["a".into(), "b".into(), "c".into()], String::new());
		let x = tape.source("x", "human", None, Fidelity::Exact, vec![Some(Cell::At(1.0)), None, Some(Cell::At(3.0))]);
		let y = tape.source("y", "human", None, Fidelity::Exact, vec![Some(Cell::At(1.0)), Some(Cell::At(0.5)), None]);
		let max = tape.reduce("max", &[x], ["max"], |c| {
			c[0].iter()
				.flatten()
				.map(|c| match c {
					Cell::At(v) => *v,
					_ => unreachable!(),
				})
				.reduce(f64::max)
				.map(|m| [m])
		});
		let v = tape.map("v", x, &[max], Some([0.0, 1.0]), |c, s| match c {
			Cell::At(x) => x / s[0],
			_ => unreachable!(),
		});
		let mean = tape.weighted_mean("mean", &[(v, 3.0), (y, 1.0)]);
		let m = tape.source("m", "human", None, Fidelity::Exact, vec![None, Some(Cell::At(0.5)), Some(Cell::At(0.25))]);
		let score = tape.gate("score", mean, &[m]);
		(tape, [x, max, v, mean, m, score])
	}

	#[test]
	fn edges_are_exactly_what_each_op_was_handed() {
		let (tape, [x, max, v, mean, m, score]) = tape();
		assert_eq!(tape.node(max).deps, [x]);
		assert_eq!(tape.node(v).deps, [x, max]);
		assert_eq!(tape.node(score).deps, [mean, m]);
	}

	#[test]
	fn a_maps_curve_passes_through_every_rows_value() {
		let (tape, [x, _, v, ..]) = tape();
		let Kind::Map { curve: Some(curve), .. } = &tape.node(v).kind else { panic!("v has a curve") };
		for (x, v) in tape.at(x).iter().zip(tape.at(v)) {
			if let (Some(x), Some(v)) = (x, v) {
				assert!(curve.contains(&[*x, v]), "({x}, {v}) is off the curve");
			}
		}
	}

	/// A value far past the domain is still on the curve, and does not thin the grid the domain is drawn from.
	#[test]
	fn a_maps_grid_spans_its_domain_rather_than_its_values() {
		let (tape, [.., v, _, _, _]) = tape();
		let Kind::Map { curve: Some(curve), domain } = &tape.node(v).kind else {
			panic!("v has a curve")
		};
		assert_eq!(*domain, Some([0.0, 1.0]));
		assert_eq!(curve.iter().filter(|p| p[0] <= 1.0).count(), SAMPLES + 1);
		assert!(curve.iter().any(|p| p[0] == 3.0), "the row past the domain is on the curve");
	}

	#[test]
	fn values_are_what_the_ops_compute() {
		let (tape, [.., mean, _, score]) = tape();
		assert_eq!(tape.at(mean), [Some((3.0 / 3.0 + 1.0) / 4.0), Some(0.5 / 4.0), Some(3.0 / 4.0)]);
		assert_eq!(tape.at(score), [Some(0.5), Some(0.5 / 4.0 * 0.5), Some(3.0 / 4.0 * 0.25)]);
	}

	#[test]
	fn a_partial_source_taints_what_reads_it() {
		let mut tape = Tape::new(vec!["a".into()], String::new());
		let x = tape.source("x", "pull", None, Fidelity::Partial("backfilling".into()), vec![Some(Cell::At(1.0))]);
		let v = tape.map("v", x, &[], None, |_, _| 1.0);
		assert_eq!(tape.node(v).fidelity, Fidelity::Partial("reads `x`".into()));
	}

	#[test]
	fn a_tape_round_trips_through_json() {
		let (tape, _) = tape();
		let json = serde_json::to_string(&tape).unwrap();
		assert_eq!(serde_json::from_str::<Tape>(&json).unwrap(), tape);
	}
}
