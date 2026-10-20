"use strict";
const D = JSON.parse(document.getElementById("data").textContent);
const T = D.tape, N = T.nodes, R = D.readings, L = D.levels, ROWS = T.rows;
const SVG = "http://www.w3.org/2000/svg";

const kind = (n) => (typeof n.kind === "string" ? n.kind : Object.keys(n.kind)[0]);
const body = (n) => (typeof n.kind === "string" ? {} : n.kind[kind(n)]);
const cells = (n) => (n.vals.Rows || null);
const point = (c) => (c && "At" in c ? c.At : null);
const pts = (x) => (100 * x).toFixed(2);
const pct = (x) => (x === null ? "·" : (100 * x).toFixed(1) + "%");
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

const el = (tag, attrs = {}, parent) => {
	const e = document.createElementNS(SVG, tag);
	for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
	if (parent) parent.appendChild(e);
	return e;
};
const text = (parent, x, y, s, attrs = {}) => {
	const t = el("text", { x, y, ...attrs }, parent);
	t.textContent = s;
	return t;
};
const tip = document.createElement("div");
tip.id = "tip";
const hover = (target, f) => {
	target.addEventListener("mousemove", (e) => {
		tip.textContent = f(e);
		tip.style.display = "block";
		tip.style.left = e.clientX + 14 + "px";
		tip.style.top = e.clientY + 10 + "px";
	});
	target.addEventListener("mouseleave", () => (tip.style.display = "none"));
};

// A term's hue is fixed by its place in its mean, never by its rank.
const hue = new Map();
for (const r of R) r.terms.forEach((t, i) => hue.set(t.of, `var(--s${(i % 8) + 1})`));
const termOf = new Map();
for (const r of R) for (const t of r.terms) termOf.set(t.of, { r, t });
const out = R.length ? (R[0].gate === null ? R[0].mean : R[0].gate) : null;
const order = out === null ? ROWS.map((_, i) => i) : ROWS.map((_, i) => i).sort((a, b) => point(cells(N[out])[b]) - point(cells(N[out])[a]));
const place = new Map(order.map((row, i) => [row, i]));
let cursor = null;

const unit = (n) => {
	if (kind(n) === "Source") return body(n).unit;
	const x = n.deps.length ? N[n.deps[0]] : null;
	return x ? unit(x) : null;
};
const fmt = (v, u) => {
	if (v === null || v === undefined) return "·";
	if (u && u.startsWith("unix seconds")) return new Date(v * 1000).toISOString().replace("T", " ").slice(0, 16);
	if (u && u.startsWith("seconds since")) {
		const d = v / 86400;
		return d >= 1 ? d.toFixed(1) + "d" : (v / 3600).toFixed(1) + "h";
	}
	return Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 1 ? v.toFixed(2) : v.toFixed(3);
};
const cellText = (c, u) => {
	if (!c) return "absent";
	if ("At" in c) return fmt(c.At, u);
	if ("Within" in c) return `${fmt(c.Within[0], u)}–${fmt(c.Within[1], u)}`;
	return `${c.Each.length} items, newest ${fmt(Math.max(...c.Each), u)}`;
};

function header() {
	const h = document.createElement("header");
	const scored = out === null ? "" : ` · ${ROWS.length} rows ranked by ${esc(N[out].name)}`;
	h.innerHTML = `<strong>${esc(N[out === null ? N.length - 1 : out].name)}</strong><span class="muted">${esc(T.stamp)}${scored}</span>
		<input id="pick" list="rows" placeholder="row… (↑/↓ through rank, Esc clears)"><datalist id="rows">${order.map((i) => `<option value="${esc(ROWS[i])}">`).join("")}</datalist>
		<span id="at" class="secondary"></span>`;
	document.body.appendChild(h);
	const pick = h.querySelector("#pick");
	pick.addEventListener("change", () => set(ROWS.indexOf(pick.value)));
	document.addEventListener("keydown", (e) => {
		if (e.key === "Escape") return set(-1);
		if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
		e.preventDefault();
		const at = cursor === null ? -1 : place.get(cursor);
		const next = Math.max(0, Math.min(order.length - 1, at + (e.key === "ArrowDown" ? 1 : -1)));
		set(order[next]);
	});
}

function set(row) {
	cursor = row >= 0 ? row : null;
	const pick = document.getElementById("pick");
	pick.value = cursor === null ? "" : ROWS[cursor];
	document.getElementById("at").textContent = cursor === null ? "cohort" : `#${place.get(cursor) + 1} of ${ROWS.length} · ${out === null ? "" : pts(point(cells(N[out])[cursor]))}`;
	draw();
}

const W = 210, H = 70, GAP_X = 80, GAP_Y = 16;
function dag(root) {
	const cols = [];
	// a mean's terms share the column left of it, so no term's edge runs through another's card
	const col = (i) => (termOf.has(i) ? L[termOf.get(i).r.mean] - 1 : L[i]);
	N.forEach((_, i) => {
		(cols[col(i)] = cols[col(i)] || []).push(i);
	});
	const at = new Map();
	cols.forEach((col, c) => (col || []).forEach((i, r) => at.set(i, { x: 10 + c * (W + GAP_X), y: 30 + r * (H + GAP_Y) })));
	const height = 40 + Math.max(...cols.map((c) => (c || []).length)) * (H + GAP_Y);
	const svg = el("svg", { width: 20 + cols.length * (W + GAP_X), height }, root);
	text(svg, 10, 16, cursor === null ? "edge width into a mean: what the term drives of its spread over the cohort" : `edge width into a mean: what the term contributes to ${ROWS[cursor]}`, { class: "muted" });

	const edges = el("g", {}, svg);
	N.forEach((n, i) => {
		n.deps.forEach((d) => {
			const a = at.get(d), b = at.get(i);
			const term = kind(n) === "WeightedMean" ? termOf.get(d) : null;
			let width = 1.2, color = null, dash = null;
			if (term) {
				color = hue.get(d);
				if (cursor === null) {
					const dr = term.t.drives;
					width = dr === null ? 1 : 1 + 14 * Math.abs(dr);
					if (dr !== null && dr < 0) dash = "4 3";
				} else width = 1 + 40 * term.t.fate[cursor].contribution;
			}
			const x1 = a.x + W, y1 = a.y + H / 2, x2 = b.x, y2 = b.y + H / 2;
			const p = el("path", { d: `M${x1},${y1} C${x1 + GAP_X / 2},${y1} ${x2 - GAP_X / 2},${y2} ${x2},${y2}`, class: "edge", "stroke-width": width }, edges);
			if (color) {
				p.style.stroke = color;
				p.style.opacity = 0.85;
			}
			if (dash) p.setAttribute("stroke-dasharray", dash);
			if (term) hover(p, () => `${n.name} ← ${N[d].name}\ndrives ${pct(term.t.drives)}${cursor === null ? "" : `\ncontributes ${pts(term.t.fate[cursor].contribution)} pts`}`);
		});
	});

	N.forEach((n, i) => {
		const { x, y } = at.get(i);
		const k = kind(n);
		const rows = cells(n);
		const never = k === "Source" && rows.every((c) => c === null);
		const g = el("g", { class: `card ${k === "Reduce" ? "reduce" : ""} ${never ? "never" : ""}`, transform: `translate(${x},${y})` }, svg);
		el("rect", { class: "box", width: W, height: H, rx: 6 }, g);
		const term = termOf.get(i);
		if (term) el("rect", { x: 0, y: 0, width: 4, height: H, rx: 2, fill: hue.get(i) }, g);
		text(g, 10, 17, n.name, { class: "title" });
		const what = k === "Source" ? `source · ${body(n).writer}` : k === "Reduce" ? "across rows → " + body(n).names.join(", ") : k === "WeightedMean" ? "weighted mean" : k === "Gate" ? "× " + (n.deps.length === 2 ? N[n.deps[1]].name : `${n.deps.length - 1} factors`) : k.toLowerCase();
		text(g, 10, 32, what, { class: "muted" });
		let line;
		if (rows === null) {
			const s = n.vals.Scalars;
			line = s === null ? "no cohort to reduce" : body(n).names.map((nm, j) => `${nm} ${fmt(s[j], nm === "width" ? "seconds since" : unit(n))}`).join(" · ");
		} else if (cursor === null) {
			const set = rows.filter((c) => c !== null).length;
			line = never ? "never set" : `${set} / ${rows.length} set`;
		} else line = cellText(rows[cursor], k === "Source" ? body(n).unit : null);
		text(g, 10, 48, line, never ? { fill: "var(--critical)" } : {});
		if (n.fidelity !== "Exact") text(g, W - 10, 17, "◐", { "text-anchor": "end" });
		if (term) {
			const max = Math.max(...term.r.terms.map((t) => t.allocated));
			const span = (W - 20) / max;
			const filled = cursor === null ? term.t.delivered : term.t.fate[cursor].delivered;
			el("rect", { x: 10, y: 56, width: term.t.allocated * span, height: 8, rx: 2, fill: "none", stroke: hue.get(i), "stroke-width": 1.5 }, g);
			el("rect", { x: 10, y: 56, width: filled * span, height: 8, rx: 2, fill: hue.get(i) }, g);
		}
		if (k === "Gate") {
			const by = n.deps.slice(1).map((d) => cells(N[d]));
			const m = ROWS.map((_, row) => by.reduce((p, c) => p * (c[row] === null ? 1 : point(c[row])), 1));
			const badge =
				cursor === null
					? `${m.filter((v) => v === 0).length} zeroed · ${m.filter((v) => v > 0 && v < 1).length} reduced`
					: by.every((c) => c[cursor] === null)
						? "not gated"
						: "× " + m[cursor].toFixed(3);
			text(g, 10, 62, badge, { class: "muted" });
		}
		hover(g, () => {
			const lines = [`${n.name} · ${k}`];
			if (n.fidelity !== "Exact") lines.push("partial: " + n.fidelity.Partial);
			if (term) lines.push(`allocated ${pct(term.t.allocated)} · delivered ${pts(term.t.delivered)} pts · drives ${pct(term.t.drives)}`);
			lines.push("reads: " + (n.deps.map((d) => N[d].name).join(", ") || "nothing"));
			return lines.join("\n");
		});
	});
}

const PW = 280, PH = 130, SH = 34, PAD = 34;
function shapes(root) {
	N.forEach((n, i) => {
		if (kind(n) !== "Map" || body(n).curve === null) return;
		const all = body(n).curve, domain = body(n).domain;
		const x = N[n.deps[0]], u = unit(x);
		const [x0, x1] = domain || [all[0][0], all[all.length - 1][0]];
		const curve = all.filter((p) => p[0] >= x0 && p[0] <= x1);
		const ys = curve.map((p) => p[1]);
		const [y0, y1] = [Math.min(0, ...ys), Math.max(1, ...ys)];
		// values past the domain pile at its edge
		const sx = (v) => PAD + (x1 === x0 ? (PW - PAD - 8) / 2 : ((Math.min(x1, Math.max(x0, v)) - x0) / (x1 - x0)) * (PW - PAD - 8));
		const sy = (v) => 22 + (1 - (v - y0) / (y1 - y0)) * (PH - 40);
		const panel = document.createElement("div");
		panel.className = "panel";
		root.appendChild(panel);
		const svg = el("svg", { width: PW, height: PH + SH + 30 }, panel);
		text(svg, 4, 13, n.name, { class: "title" });
		const color = hue.get(i) || "var(--text-secondary)";
		const grid = el("g", { class: "grid" }, svg);
		for (const v of [y0, y1]) {
			el("line", { x1: PAD, x2: PW - 8, y1: sy(v), y2: sy(v) }, grid);
			text(svg, PAD - 4, sy(v) + 4, fmt(v), { "text-anchor": "end" });
		}
		el("polyline", { points: curve.map((p) => `${sx(p[0])},${sy(p[1])}`).join(" "), fill: "none", stroke: color, "stroke-width": 2 }, svg);

		// the slope on an axis of its own, below rather than over the curve
		const slope = [];
		for (let j = 1; j < curve.length; j++) {
			const dx = curve[j][0] - curve[j - 1][0];
			if (dx > 0) slope.push([(curve[j][0] + curve[j - 1][0]) / 2, (curve[j][1] - curve[j - 1][1]) / dx]);
		}
		const top = PH - 10, smax = Math.max(1e-300, ...slope.map((s) => Math.abs(s[1])));
		const ss = (v) => top + SH / 2 - (v / smax) * (SH / 2 - 2);
		el("line", { x1: PAD, x2: PW - 8, y1: ss(0), y2: ss(0), stroke: "var(--line)" }, svg);
		el("polyline", { points: slope.map((s) => `${sx(s[0])},${ss(s[1])}`).join(" "), fill: "none", stroke: color, "stroke-width": 1, "stroke-dasharray": "3 2" }, svg);
		text(svg, PAD - 4, ss(0) + 4, "slope", { "text-anchor": "end", class: "muted" });

		const rug = el("g", { opacity: 0.45 }, svg);
		const base = top + SH + 4;
		const past = [0, 0];
		for (const c of cells(x)) {
			if (!c) continue;
			const vs = "At" in c ? [c.At] : "Within" in c ? c.Within : c.Each;
			if (vs.every((v) => v < x0)) past[0]++;
			if (vs.every((v) => v > x1)) past[1]++;
			if ("At" in c) el("line", { x1: sx(c.At), x2: sx(c.At), y1: base, y2: base + 8, stroke: "var(--text-secondary)" }, rug);
			else if ("Within" in c) el("line", { x1: sx(c.Within[0]), x2: sx(c.Within[1]), y1: base + 4, y2: base + 4, stroke: "var(--text-secondary)" }, rug);
			else for (const t of c.Each) el("line", { x1: sx(t), x2: sx(t), y1: base, y2: base + 8, stroke: "var(--text-secondary)" }, rug);
		}
		const beyond = (n, side) => `${n} row${n === 1 ? "" : "s"} ${side} ${fmt(side === "below" ? x0 : x1, u)}, piled at the edge`;
		if (past[0]) hover(text(svg, PAD, base - 2, `‹${past[0]}`, { fill: "var(--critical)" }), () => beyond(past[0], "below"));
		if (past[1]) hover(text(svg, PW - 8, base - 2, `${past[1]}›`, { fill: "var(--critical)", "text-anchor": "end" }), () => beyond(past[1], "above"));
		text(svg, PAD, base + 18, fmt(x0, u));
		text(svg, PW - 8, base + 18, fmt(x1, u), { "text-anchor": "end" });
		if (u) text(svg, (PAD + PW - 8) / 2, base + 30, u, { class: "muted", "text-anchor": "middle" });

		if (cursor !== null) {
			const c = cells(x)[cursor], v = point(cells(n)[cursor]);
			const dot = (cx, cy) => el("circle", { cx, cy, r: 4.5, fill: color, stroke: "var(--surface-1)", "stroke-width": 2 }, svg);
			if (c && "At" in c) dot(sx(c.At), sy(v)).setAttribute("fill-opacity", c.At < x0 || c.At > x1 ? 0.3 : 1);
			else if (c && "Within" in c) el("line", { x1: sx(c.Within[0]), x2: sx(c.Within[1]), y1: sy(v), y2: sy(v), stroke: color, "stroke-width": 4, "stroke-linecap": "round" }, svg);
			else if (c && "Each" in c) {
				const at = new Map(all);
				for (const t of c.Each) dot(sx(t), sy(at.get(t)));
			}
		}
		const catcher = el("rect", { x: PAD, y: 18, width: PW - PAD - 8, height: PH + SH - 10, fill: "transparent" }, svg);
		hover(catcher, (e) => {
			const r = catcher.getBoundingClientRect();
			const want = x0 + ((e.clientX - r.left) / r.width) * (x1 - x0);
			let best = curve[0];
			for (const p of curve) if (Math.abs(p[0] - want) < Math.abs(best[0] - want)) best = p;
			return `${fmt(best[0], u)} → ${fmt(best[1])}`;
		});
	});
}

function points(root) {
	for (const r of R) {
		const mean = N[r.mean];
		const at = (t, f) => (cursor === null ? t.fate.reduce((s, x) => s + f(x), 0) / ROWS.length : f(t.fate[cursor]));
		const loss = (f) => (cursor === null ? f.loss.reduce((s, x) => s + x, 0) / ROWS.length : f.loss[cursor]);
		const segs = [];
		for (const t of r.terms) {
			const c = hue.get(t.of), name = N[t.of].name;
			segs.push({ v: at(t, (x) => x.contribution), fill: c, name, what: "delivered" });
			segs.push({ v: at(t, (x) => x.shape_loss), fill: c, opacity: 0.25, name, what: "shape loss" });
			segs.push({ v: at(t, (x) => x.absent), fill: "none", name, what: "absent", edge: c, dash: "3 2" });
		}
		for (const f of r.factors) segs.push({ v: loss(f), fill: "url(#gateloss)", name: N[f.of].name, what: "gate loss", edge: "var(--critical)" });
		const total = segs.filter((s) => s.what === "delivered").reduce((s, x) => s + x.v, 0);
		const BW = Math.max(700, Math.min(1400, window.innerWidth - 80)), BH = 34;
		const panel = document.createElement("div");
		panel.className = "panel";
		root.appendChild(panel);
		const who = cursor === null ? "the cohort's mean row" : ROWS[cursor];
		const p = document.createElement("div");
		p.innerHTML = `<strong>${esc(mean.name)}${r.factors.map((f) => " × " + esc(N[f.of].name)).join("")}</strong> <span class="secondary">— the 100 points of ${esc(who)}: ${pts(total)} delivered</span>`;
		panel.appendChild(p);
		const svg = el("svg", { width: BW + 2, height: BH + 26 }, panel);
		const defs = el("defs", {}, svg);
		const pat = el("pattern", { id: "gateloss", width: 6, height: 6, patternUnits: "userSpaceOnUse", patternTransform: "rotate(45)" }, defs);
		el("rect", { width: 6, height: 6, fill: "var(--surface-1)" }, pat);
		el("line", { x1: 0, y1: 0, x2: 0, y2: 6, stroke: "var(--critical)", "stroke-width": 2.5 }, pat);
		let x = 1;
		for (const s of segs) {
			const w = s.v * BW;
			if (w <= 0) continue;
			const r2 = el("rect", { x: x + 1, y: 2, width: Math.max(0, w - 2), height: BH, rx: 2, fill: s.fill }, svg);
			if (s.opacity) r2.setAttribute("fill-opacity", s.opacity);
			if (s.edge) {
				r2.setAttribute("stroke", s.edge);
				r2.setAttribute("stroke-width", 1);
			}
			if (s.dash) r2.setAttribute("stroke-dasharray", s.dash);
			hover(r2, () => `${s.name} · ${s.what}\n${pts(s.v)} pts`);
			if (w > 34 && s.what === "delivered") text(svg, x + w / 2, BH + 16, pts(s.v), { "text-anchor": "middle" });
			if (w > 60 && s.what === "gate loss") text(svg, x + w / 2, BH + 16, `${s.name} ${pts(s.v)}`, { "text-anchor": "middle", fill: "var(--critical)" });
			x += w;
		}
		const legend = document.createElement("div");
		legend.className = "legend";
		legend.innerHTML =
			r.terms.map((t) => `<span><span class="swatch" style="background:${hue.get(t.of)}"></span>${esc(N[t.of].name)}</span>`).join("") +
			`<span class="muted">solid = delivered · pale = shape loss · dashed outline = absent · hatched red = gate loss, per factor (its Shapley share of the product's)</span>`;
		panel.appendChild(legend);

		const rows = r.terms
			.map((t) => {
				const set = cells(N[t.of]).filter((c) => c !== null).length;
				const here = cursor === null ? "" : `<td class="num">${fmt(point(cells(N[t.of])[cursor]))}</td><td class="num">${pts(t.fate[cursor].contribution)}</td><td class="num">${pts(t.fate[cursor].shape_loss + t.fate[cursor].absent)}</td>`;
				return `<tr><td><span class="swatch" style="background:${hue.get(t.of)}"></span>${esc(N[t.of].name)}</td><td class="num">${pts(t.allocated)}</td><td class="num">${pts(t.delivered)}</td><td class="num">${pct(t.drives)}</td><td class="num">${set}</td>${here}</tr>`;
			})
			.join("");
		const head = `<tr><th>term</th><th>allocated</th><th>delivered</th><th>drives</th><th>set</th>${cursor === null ? "" : "<th>v</th><th>contribution</th><th>headroom</th>"}</tr>`;
		const gateRows = r.factors
			.map((f) => {
				const c = cells(N[f.of]);
				const here = cursor === null ? "" : `<td class="num">${fmt(point(c[cursor]))}</td><td class="num">−${pts(f.loss[cursor])}</td><td></td>`;
				return `<tr><td class="muted">gate loss · ${esc(N[f.of].name)}</td><td></td><td class="num">−${pts(loss(f))}</td><td></td><td class="num">${c.filter((v) => v !== null).length}</td>${here}</tr>`;
			})
			.join("");
		const table = document.createElement("table");
		table.innerHTML = head + rows + gateRows;
		panel.appendChild(table);
	}
}

function draw() {
	for (const id of ["dag", "shapes", "points"]) document.getElementById(id).replaceChildren();
	dag(document.getElementById("dag"));
	shapes(document.getElementById("shapes"));
	points(document.getElementById("points"));
}

header();
document.body.insertAdjacentHTML(
	"beforeend",
	`<h2>Graph</h2><div class="panel" id="dag"></div><h2>Shapes</h2><div class="strip" id="shapes"></div><h2>Points</h2><div id="points"></div>`,
);
document.body.appendChild(tip);
set(ROWS.indexOf(decodeURIComponent(location.hash.slice(1))));
