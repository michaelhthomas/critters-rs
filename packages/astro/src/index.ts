import fs from "node:fs/promises";
import path from "node:path";
import url from "node:url";
import c from "chalk";

import { Critters, type CrittersOptions } from "@critters-rs/critters";
import type { AstroConfig, AstroIntegration } from "astro";

/**
 * Recursively locate all .html files in a given directory
 * @param {string} dir - The directory to search in
 * @param {Array} fileList - Used to store the found .html files
 * @returns {Array} List of .html file paths
 */
async function locateHtmlFiles(dir: string, fileList: string[] = []) {
	const files = await fs.readdir(dir);

	for (const file of files) {
		const filePath = path.join(dir, file);
		const stat = await fs.stat(filePath);

		if (stat.isDirectory()) {
			locateHtmlFiles(filePath, fileList);
		} else if (path.extname(file).toLowerCase() === ".html") {
			fileList.push(filePath);
		}
	}

	return fileList;
}

/**
 * Formats the elapsed time from an initial measurement captured with
 * `process.hrtime`. Will output the duration in seconds if more than one
 * second has passed, otherwise in milliseconds.
 */
function elapsed(hrtime: [number, number]) {
	const [s, ns] = process.hrtime(hrtime);
	if (s >= 1) {
		const secs = s + ns / 10 ** 9;
		return `${secs.toFixed(2)}s`;
	}
	const ms = ns / 10 ** 6;
	return `${Math.round(ms)}ms`;
}

type AstroCrittersOptions = Omit<CrittersOptions, "path">;

export default (options?: AstroCrittersOptions): AstroIntegration => {
	let config: AstroConfig;

	return {
		name: "@critters-rs/astro",
		hooks: {
			"astro:config:done": ({ config: cfg }) => {
				config = cfg;
			},
			"astro:build:done": async ({ dir, logger: mainLogger }) => {
				const logger = mainLogger.fork("critters");
				logger.info(c.bgGreen(" inlining critical css "));
				const start = process.hrtime();

				const dist = url.fileURLToPath(dir);
				const paths = await locateHtmlFiles(dist);

				// infer public path for CSS assets from Astro config
				const publicPath = config.build.assetsPrefix
					? typeof config.build.assetsPrefix === "string"
						? config.build.assetsPrefix
						: (config.build.assetsPrefix.css ??
							config.build.assetsPrefix.fallback)
					: undefined;

				logger.debug(`resolved public path: ${publicPath}`);

				const critters = new Critters({
					publicPath,
					external: true,
					...options,
					path: dist,
				});

				for (const [i, path] of paths.entries()) {
					const pageStart = process.hrtime();

					const html = await fs.readFile(path).then((b) => b.toString("utf-8"));
					const result = critters.process(html);
					await fs.writeFile(path, result);

					const time = process.hrtime(pageStart);
					const durationColor =
						time[0] > 0 ? c.red : time[1] > 10 ** 8 ? c.yellow : c.grey;
					logger.info(
						[
							c.blue(i === paths.length - 1 ? " └─" : " ├─"),
							c.gray(path.replace(dist, "")),
							durationColor(`(+${elapsed(pageStart)})`),
						].join(" "),
					);
				}

				logger.info(c.green(`✓ Completed in ${elapsed(start)}.`));
			},
		},
	};
};
