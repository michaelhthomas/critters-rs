import fs from "node:fs/promises";
import path from "node:path";
import util from "node:util";
import url from "node:url";
import { spawn } from "node:child_process";
import { NapiCli } from "@napi-rs/cli";
import { rimraf } from "rimraf";
import { consola as log } from "consola";

import { rollup } from "rollup";
import _pluginCjs from "@rollup/plugin-commonjs";
import _pluginEsmShim from "@rollup/plugin-esm-shim";
import _pluginCopy from "rollup-plugin-copy";
// see https://github.com/rollup/plugins/issues/1662
const pluginCjs = _pluginCjs as unknown as typeof _pluginCjs.default;
const pluginEsmShim =
	_pluginEsmShim as unknown as typeof _pluginEsmShim.default;
const pluginCopy = _pluginCopy as unknown as typeof _pluginCopy.default;

const JS_DIR = path.dirname(url.fileURLToPath(import.meta.url));
const CRATE_PATH = path.resolve(JS_DIR, "../..");
const RUST_OUT_DIR = path.join(JS_DIR, "./pkg");

const cli = new NapiCli();
const args = util.parseArgs({
	options: {
		target: {
			type: "string",
		},
		"use-napi-cross": {
			type: "boolean",
		},
	},
});

// rust build
log.start("Building crate for NAPI...");
await cli
	.build({
		manifestPath: path.join(CRATE_PATH, "Cargo.toml"),
		features: ["use-napi"],
		cargoOptions: ["--lib"],
		release: true,
		platform: true,
		target: args.values.target,
		useNapiCross: args.values["use-napi-cross"],
		outputDir: RUST_OUT_DIR,
	})
	.then((out) => out.task);
log.success("Build complete!");

// fix types
log.start("Updating bindings...");
await new Promise<void>((resolve, reject) => {
	const cargo = process.env.CARGO ?? "cargo";
	const bindingsProcess = spawn(
		cargo,
		"test export_bindings --lib --features typegen".split(" "),
		{
			cwd: CRATE_PATH,
			env: { ...process.env, TS_RS_EXPORT_DIR: RUST_OUT_DIR },
			stdio: "inherit",
		},
	);

	bindingsProcess.once("exit", (code) => {
		if (code === 0) {
			resolve();
		} else {
			reject(new Error(`Bindings generation failed with exit code ${code}`));
		}
	});

	bindingsProcess.once("error", (e) => {
		reject(
			new Error(`Bindings generation failed with error: ${e.message}`, {
				cause: e,
			}),
		);
	});
});

const declarationFile = path.join(RUST_OUT_DIR, "index.d.ts");
const declaration = await fs
	.readFile(declarationFile)
	.then((b) => b.toString("utf-8"));
const updatedDeclaration = `import type { CrittersOptions as FullCrittersOptions } from "./CrittersOptions.ts";
export type CrittersOptions = Optional<FullCrittersOptions>;

${declaration.replace(/constructor\(.*?\)/, "constructor(options?: CrittersOptions)")}
`;
await fs.writeFile(declarationFile, updatedDeclaration);
log.success("Bindings updated");

// bundle
log.start("Bundling to ESM...");
const output = await rollup({
	input: path.join(JS_DIR, "index.js"),
	external: ["util", "path", "fs", /.*\.node$/],
	plugins: [
		pluginCjs({ defaultIsModuleExports: false }),
		pluginEsmShim(),
		pluginCopy({
			targets:
				// if the NAPI_TEST environment variable is set, we use the .node file
				// from the project directory (usually generated using pipelines),
				// instead of using the one created while generating the bindings.
				process.env.NAPI_TEST === "true"
					? [
							{ src: "pkg/*.ts", dest: "dist" },
							{ src: "../../*.node", dest: "dist" },
						]
					: [{ src: "pkg/*.{node,ts}", dest: "dist" }],
		}),
	],
});

await output.write({
	dir: "dist",
	format: "esm",
});
log.success("Bundling complete!");

// clean up intermediary files
log.start("Cleaning up...");
await rimraf(RUST_OUT_DIR);
log.success("Complete!");
