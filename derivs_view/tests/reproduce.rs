//! A tape is self-sufficient: the page of a saved tape is the page of the tape it was saved from.

use derivs::{Cell, Fidelity, Tape};

#[test]
fn a_saved_tape_renders_to_the_same_page() {
	let mut tape = Tape::new(vec!["a".into(), "b".into()], "test".into());
	let x = tape.source("x", "human", Some("km"), Fidelity::Exact, vec![Some(Cell::At(0.1 + 0.2)), Some(Cell::Within(1.0 / 3.0, 2.0))]);
	let v = tape.map("v", x, &[], Some([0.0, 2.0]), |c, _| match c {
		Cell::At(x) => (-x).exp(),
		Cell::Within(lo, hi) => ((-lo).exp() + (-hi).exp()) / 2.0,
		Cell::Each(_) => unreachable!(),
	});
	tape.weighted_mean("mean", &[(v, 3.0)]);
	let saved: Tape = serde_json::from_str(&serde_json::to_string(&tape).unwrap()).unwrap();
	assert_eq!(derivs_view::html(&saved), derivs_view::html(&tape));
}
