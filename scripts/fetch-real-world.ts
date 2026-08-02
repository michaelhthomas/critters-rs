/**
 * Fetch real-world pages and their CSS into source-controlled fixtures, then
 * regenerate the Rust site list consumed by the benchmark and snapshot-test
 * harnesses.
 *
 *   pnpm benchmarks:fetch              # fetch only sites not yet on disk (default)
 *   pnpm benchmarks:fetch --all        # refetch every site in the manifest
 *   pnpm benchmarks:fetch <name>...    # (re)fetch specific sites by name
 *
 * For each site in `test_files/manifest.jsonc` this:
 *   1. fetches the page HTML,
 *   2. downloads every linked `<link rel="stylesheet">` CSS file (following
 *      `@import` chains) into `test_files/<name>/assets/`,
 *   3. rewrites the `<link>` hrefs (and `@import` targets) to the local files,
 *   4. writes `test_files/<name>/index.html`,
 *   5. regenerates `test_files/sites.gen.rs`.
 *
 * After fetching, create/update snapshots with:
 *   cargo insta test --accept -- <name>
 */

import { DOMParser } from "jsr:@b-fuze/deno-dom";
import { crypto } from "jsr:@std/crypto";
import { encodeHex } from "jsr:@std/encoding/hex";
import { parse as parseJSONC } from "jsr:@std/jsonc";
import { join, resolve } from "jsr:@std/path";

const REPO_ROOT = resolve(import.meta.dirname ?? ".", "..");
const TEST_FILES = join(REPO_ROOT, "test_files");
const MANIFEST = join(TEST_FILES, "manifest.jsonc");
const SITES_GEN = join(TEST_FILES, "sites.gen.rs");

type Site = {
	name: string;
	url: string;
};

const RUST_IDENT = /^[A-Za-z_][A-Za-z0-9_]*$/;

async function readManifest(): Promise<Site[]> {
	const json = await Deno.readTextFile(MANIFEST);
	const sites = parseJSONC(json) as Site[];
	for (const site of sites) {
		if (!site.name || !site.url) {
			throw new Error(
				`Manifest entry missing name/url: ${JSON.stringify(site)}`,
			);
		}
		if (!RUST_IDENT.test(site.name)) {
			throw new Error(
				`Site name "${site.name}" is not a valid Rust identifier (used as a test fn / benchmark id / snapshot name).`,
			);
		}
	}
	return sites;
}

async function fetchText(url: string): Promise<string> {
	const res = await fetch(url, {
		headers: {
			"user-agent":
				"Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
			accept: "text/html,text/css,*/*",
		},
	});
	if (!res.ok) {
		throw new Error(`GET ${url} -> ${res.status} ${res.statusText}`);
	}
	return res.text();
}

/** Turn a stylesheet URL into a stable, unique, `.css` local filename. */
async function assetFilename(
	absUrl: string,
	taken: Map<string, string>,
): Promise<string> {
	const existing = taken.get(absUrl);
	if (existing) return existing;

	const { pathname } = new URL(absUrl);
	let base = pathname.split("/").pop() || "style";
	base = base.replace(/[^A-Za-z0-9._-]/g, "_");
	if (!base.toLowerCase().endsWith(".css")) base += ".css";

	let name = base;
	// Disambiguate collisions with a short hash of the URL.
	if ([...taken.values()].includes(name)) {
		const hash = encodeHex(
			await crypto.subtle.digest("SHA-1", new TextEncoder().encode(absUrl)),
		).slice(0, 8);
		name = `${base.slice(0, -4)}.${hash}.css`;
	}
	taken.set(absUrl, name);
	return name;
}

/**
 * Download a stylesheet and, recursively, any sheets it `@import`s. Rewrites
 * `@import` targets to the local relative filenames. Returns the (possibly
 * rewritten) CSS to write for `absUrl`.
 */
async function fetchStylesheet(
	absUrl: string,
	taken: Map<string, string>,
	contents: Map<string, string>,
): Promise<void> {
	if (contents.has(absUrl)) return;
	contents.set(absUrl, ""); // reserve slot to break import cycles

	let css: string;
	try {
		css = await fetchText(absUrl);
	} catch (err) {
		console.warn(
			`  ! skipping stylesheet ${absUrl}: ${(err as Error).message}`,
		);
		contents.delete(absUrl);
		return;
	}

	// Resolve and follow @import statements: `@import "x.css"` / `@import url(x.css)`.
	const importRe =
		/@import\s+(?:url\(\s*)?["']?([^"')]+?)["']?\s*\)?(\s*[^;]*)?;/g;
	const imports = [...css.matchAll(importRe)].map((m) => ({
		raw: m[0],
		target: m[1].trim(),
		suffix: (m[2] ?? "").trim(),
	}));

	for (const imp of imports) {
		if (imp.target.startsWith("data:")) continue;
		let importUrl: string;
		try {
			importUrl = new URL(imp.target, absUrl).toString();
		} catch {
			continue;
		}
		if (!/^https?:/.test(importUrl)) continue;
		await fetchStylesheet(importUrl, taken, contents);
		const localName = taken.get(importUrl);
		if (localName) {
			const media = imp.suffix ? ` ${imp.suffix}` : "";
			css = css.replace(imp.raw, `@import "${localName}"${media};`);
		}
	}

	await assetFilename(absUrl, taken); // ensure a filename is reserved
	contents.set(absUrl, css);
}

async function processSite(site: Site): Promise<void> {
	console.log(`• ${site.name}  <-  ${site.url}`);
	const siteDir = join(TEST_FILES, site.name);
	const assetsDir = join(siteDir, "assets");

	// Clean old assets
	try {
		await Deno.remove(siteDir, { recursive: true });
	} catch (err) {
		if (!(err instanceof Deno.errors.NotFound)) throw err;
	}
	await Deno.mkdir(assetsDir, { recursive: true });

	const html = await fetchText(site.url);
	const root = new DOMParser().parseFromString(html, "text/html");

	const taken = new Map<string, string>(); // absUrl -> local filename
	const contents = new Map<string, string>(); // absUrl -> css text

	const links = [...root.querySelectorAll("link")].filter((l) =>
		(l.getAttribute("rel") ?? "").split(/\s+/).includes("stylesheet"),
	);

	for (const link of links) {
		const href = link.getAttribute("href");
		if (!href) continue;
		let absUrl: string;
		try {
			absUrl = new URL(href, site.url).toString();
		} catch {
			continue;
		}
		if (!/^https?:/.test(absUrl)) continue; // skip data:/local/other schemes

		await fetchStylesheet(absUrl, taken, contents);
		const localName = taken.get(absUrl);
		if (localName && contents.has(absUrl)) {
			link.setAttribute("href", `assets/${localName}`);
		}
	}

	// Write stylesheets (sorted by URL for deterministic diffs).
	let sheetCount = 0;
	for (const [absUrl, css] of [...contents.entries()].sort((a, b) =>
		a[0].localeCompare(b[0]),
	)) {
		if (!css.length) continue;
		const localName = taken.get(absUrl);
		if (!localName) continue;
		await Deno.writeTextFile(join(assetsDir, localName), css);
		sheetCount++;
	}

	let out = `<!DOCTYPE html>${root.documentElement!.outerHTML}`;
	if (!out.endsWith("\n")) out += "\n";
	console.log(siteDir);
	await Deno.writeTextFile(join(siteDir, "index.html"), out);

	console.log(`  wrote index.html + ${sheetCount} stylesheet(s)`);
}

function renderSitesGen(names: string[]): string {
	const sorted = [...names].sort();
	const body = sorted.map((n) => `    ${n},`).join("\n");
	return [
		"// @generated by scripts/fetch-real-world.ts from test_files/manifest.jsonc",
		"// Do not edit by hand. Run `pnpm benchmarks:fetch` to regenerate.",
		"real_world_sites! {",
		body,
		"}",
		"",
	].join("\n");
}

/** Whether a site's fixtures already exist on disk. */
async function isFetched(site: Site): Promise<boolean> {
	try {
		await Deno.stat(join(TEST_FILES, site.name, "index.html"));
		return true;
	} catch (err) {
		if (err instanceof Deno.errors.NotFound) return false;
		throw err;
	}
}

async function main() {
	const args = Deno.args;
	const all = args.includes("--all");
	const names = args.filter((a) => !a.startsWith("--"));

	const manifest = await readManifest();

	if (names.length) {
		const missing = names.filter((n) => !manifest.some((s) => s.name === n));
		if (missing.length) {
			throw new Error(`Unknown site(s): ${missing.join(", ")}`);
		}
	}

	let selected: Site[];
	if (names.length) {
		// Explicitly named sites are always (re)fetched.
		selected = manifest.filter((s) => names.includes(s.name));
	} else if (all) {
		// Refetch everything.
		selected = manifest;
	} else {
		// Default: only sites that haven't been fetched to the filesystem yet.
		selected = [];
		for (const site of manifest) {
			if (!(await isFetched(site))) selected.push(site);
		}
	}

	if (selected.length === 0) {
		console.log(
			"All manifest sites are already fetched. Pass --all to refetch every site, or a site name to refetch specific ones.",
		);
	}

	for (const site of selected) {
		await processSite(site);
	}

	// The generated list always reflects the full manifest, even on a partial fetch.
	await Deno.writeTextFile(
		SITES_GEN,
		renderSitesGen(manifest.map((s) => s.name)),
	);
	console.log(`\nRegenerated ${SITES_GEN.replace(`${REPO_ROOT}/`, "")}`);

	if (selected.length) {
		const fetched = selected.map((s) => s.name).join(" ");
		console.log(
			`\nNext: create/update snapshots with\n  cargo insta test --accept -- ${fetched}`,
		);
	}
}

main().catch((err) => {
	console.error(err);
	Deno.exit(1);
});
