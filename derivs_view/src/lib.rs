//! A tape as one self-contained html page. The page reads nothing but what is embedded in it.

use derivs::Tape;

/// The tape, its readings and its layout embedded as JSON, under one vanilla-JS renderer.
pub fn html(tape: &Tape) -> String {
	let data = serde_json::json!({ "tape": tape, "readings": derivs::read(tape), "levels": derivs::levels(tape) });
	// `</` would close the script element the JSON sits in
	let data = serde_json::to_string(&data).expect("a tape is plain data").replace("</", "<\\/");
	format!(
		"<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n<style>\n{css}</style>\n</head>\n<body>\n<script type=\"application/json\" id=\"data\">{data}</script>\n<script>\n{js}</script>\n</body>\n</html>\n",
		title = tape.stamp.replace('&', "&amp;").replace('<', "&lt;"),
		css = include_str!("view.css"),
		js = include_str!("view.js"),
	)
}
