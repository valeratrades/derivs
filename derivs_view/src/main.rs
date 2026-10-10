use std::path::PathBuf;

/// `derivs view <tape.json>`: the page for a saved tape, written beside it as `<tape>.html`.
fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let [cmd, path] = args.as_slice() else {
		eprintln!("usage: derivs view <tape.json>");
		std::process::exit(2);
	};
	assert_eq!(cmd, "view", "the one command is `view`");
	let path = PathBuf::from(path);
	let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
	let tape: derivs::Tape = serde_json::from_str(&json).unwrap_or_else(|e| panic!("{} is no tape: {e}", path.display()));
	let out = path.with_extension("html");
	std::fs::write(&out, derivs_view::html(&tape)).unwrap_or_else(|e| panic!("writing {}: {e}", out.display()));
	println!("{}", out.display());
}
