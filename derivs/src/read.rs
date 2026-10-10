//! What a tape says about itself, off its node kinds alone.

use serde::{Deserialize, Serialize};

use crate::{Id, Kind, Tape};

/// One `WeightedMean`, and the `Gate` over it if there is one.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Reading {
	pub mean: Id,
	/// The gate node, and its multiplier.
	pub gate: Option<(Id, Id)>,
	pub terms: Vec<Term>,
	/// Per row, the points the gate took off the mean.
	pub gate_loss: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Term {
	pub of: Id,
	/// `w / Σw`.
	pub allocated: f64,
	/// The cohort mean of `w·v / Σw`.
	pub delivered: f64,
	/// `cov(w·v/Σw, mean) / var(mean)`: how much of the mean's spread over the cohort is this term's. `None` when the mean does not spread.
	pub drives: Option<f64>,
	/// Per row.
	pub fate: Vec<Fate>,
}

/// One row's share of a term, in points of the mean: `delivered + shape_loss + absent = w / Σw`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Fate {
	/// `w·v / Σw`.
	pub delivered: f64,
	/// `w·(1 − v) / Σw` of a value it holds.
	pub shape_loss: f64,
	/// `w / Σw` when it holds none.
	pub absent: f64,
	/// `delivered` × the gate's multiplier: exactly its Shapley value against the all-absent row.
	pub contribution: f64,
}

pub fn read(tape: &Tape) -> Vec<Reading> {
	let nodes = tape.nodes();
	nodes
		.iter()
		.enumerate()
		.filter_map(|(i, node)| match &node.kind {
			Kind::WeightedMean { weights } => Some((Id(i as u32), node, weights)),
			_ => None,
		})
		.map(|(mean, node, weights)| {
			let gate = nodes
				.iter()
				.enumerate()
				.find(|(_, n)| n.kind == Kind::Gate && n.deps[0] == mean)
				.map(|(i, n)| (Id(i as u32), n.deps[1]));
			let multiplier: Vec<f64> = match gate {
				Some((_, by)) => tape.at(by).into_iter().map(|m| m.unwrap_or(1.0)).collect(),
				None => vec![1.0; tape.rows.len()],
			};
			let out: Vec<f64> = tape.at(gate.map_or(mean, |(g, _)| g)).into_iter().map(|v| v.expect("a mean is on every row")).collect();
			let means: Vec<f64> = tape.at(mean).into_iter().map(|v| v.expect("a mean is on every row")).collect();
			let total: f64 = weights.iter().sum();
			let terms: Vec<Term> = node
				.deps
				.iter()
				.zip(weights)
				.map(|(of, w)| {
					let share = w / total;
					let fate: Vec<Fate> = tape
						.at(*of)
						.into_iter()
						.zip(&multiplier)
						.map(|(v, m)| match v {
							Some(v) => Fate {
								delivered: share * v,
								shape_loss: share * (1.0 - v),
								absent: 0.0,
								contribution: m * share * v,
							},
							None => Fate {
								delivered: 0.0,
								shape_loss: 0.0,
								absent: share,
								contribution: 0.0,
							},
						})
						.collect();
					let delivered: Vec<f64> = fate.iter().map(|f| f.delivered).collect();
					Term {
						of: *of,
						allocated: share,
						delivered: mean_of(&delivered),
						drives: (variance(&means) > 0.0).then(|| covariance(&delivered, &means) / variance(&means)),
						fate,
					}
				})
				.collect();
			let gate_loss: Vec<f64> = means.iter().zip(&multiplier).map(|(v, m)| v * (1.0 - m)).collect();
			for (i, row) in tape.rows.iter().enumerate() {
				let sum = |f: fn(&Fate) -> f64| terms.iter().map(|t| f(&t.fate[i])).sum::<f64>();
				let close = |a: f64, b: f64, what: &str| assert!((a - b).abs() <= 1e-12, "`{}` at {row}: {what} sums to {a}, and is {b}", node.name);
				close(sum(|f| f.contribution), out[i], "the contributions");
				close(sum(|f| f.delivered) - gate_loss[i], out[i], "delivered less the gate's loss");
				close(sum(|f| f.delivered + f.shape_loss + f.absent), 1.0, "every fate");
			}
			Reading { mean, gate, terms, gate_loss }
		})
		.collect()
}

/// Per node, its column: `max(1 + level(dep), level(gate))`, a gate's multiplier being drawn on the gated node's card rather than left of it.
pub fn levels(tape: &Tape) -> Vec<usize> {
	let mut levels: Vec<usize> = Vec::with_capacity(tape.nodes().len());
	for node in tape.nodes() {
		let at = |id: &Id| levels[id.0 as usize];
		let level = match node.kind {
			Kind::Gate => (at(&node.deps[0]) + 1).max(at(&node.deps[1])),
			_ => node.deps.iter().map(|d| at(d) + 1).max().unwrap_or(0),
		};
		levels.push(level);
	}
	levels
}

fn mean_of(xs: &[f64]) -> f64 {
	xs.iter().sum::<f64>() / xs.len() as f64
}

fn covariance(xs: &[f64], ys: &[f64]) -> f64 {
	let (mx, my) = (mean_of(xs), mean_of(ys));
	xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)).sum::<f64>() / xs.len() as f64
}

fn variance(xs: &[f64]) -> f64 {
	covariance(xs, xs)
}

#[cfg(test)]
mod tests {
	use crate::{Cell, Fidelity, Tape, levels, read};

	/// a: x = 1, gated ×0.5; b: x absent; c: x = 0.5. Weights 3 and 1, the second term always 1.
	fn check() -> (Tape, Vec<crate::Reading>) {
		let mut tape = Tape::new(vec!["a".into(), "b".into(), "c".into()], String::new());
		let x = tape.source("x", "human", None, Fidelity::Exact, vec![Some(Cell::At(1.0)), None, Some(Cell::At(0.5))]);
		let one = tape.source("one", "human", None, Fidelity::Exact, vec![Some(Cell::At(1.0)); 3]);
		let mean = tape.weighted_mean("mean", &[(x, 3.0), (one, 1.0)]);
		let m = tape.source("m", "human", None, Fidelity::Exact, vec![Some(Cell::At(0.5)), None, None]);
		tape.gate("score", mean, m);
		let readings = read(&tape);
		(tape, readings)
	}

	#[test]
	fn a_rows_points_split_into_delivered_shape_absent_and_gate() {
		let (_, readings) = check();
		let r = &readings[0];
		assert_eq!(r.terms[0].allocated, 0.75);
		let a = &r.terms[0].fate[0];
		assert_eq!((a.delivered, a.contribution), (0.75, 0.375));
		assert_eq!(r.terms[0].fate[1].absent, 0.75);
		assert_eq!(r.terms[0].fate[2].shape_loss, 0.375);
		assert_eq!(r.gate_loss, [0.5, 0.0, 0.0]);
	}

	#[test]
	fn a_term_everybody_gets_alike_drives_nothing() {
		let (_, readings) = check();
		assert_eq!(readings[0].terms[1].drives, Some(0.0));
		assert_eq!(readings[0].terms[0].drives, Some(1.0));
	}

	/// The multiplier is two maps deep, the mean one: drawn on the gated card, it pulls that card to its own column rather than one past it.
	#[test]
	fn a_gates_multiplier_docks_rather_than_pushing_right() {
		let mut tape = Tape::new(vec!["a".into()], String::new());
		let x = tape.source("x", "human", None, Fidelity::Exact, vec![Some(Cell::At(1.0))]);
		let mean = tape.weighted_mean("mean", &[(x, 1.0)]);
		let half = tape.map("half", x, &[], None, |_, _| 0.5);
		let m = tape.map("m", half, &[], None, |_, _| 0.5);
		tape.gate("score", mean, m);
		assert_eq!(levels(&tape), [0, 1, 1, 2, 2]);
	}
}
