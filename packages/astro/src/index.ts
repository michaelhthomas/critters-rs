import url from "node:url";
import c from "chalk";

import { Critters, type CrittersOptions } from "@critters-rs/critters";
import type { AstroConfig, AstroIntegration } from "astro";

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

				const dist = url.fileURLToPath(dir);

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

				const stats = critters.processDir();

				logger.info(c.green(` ✓ Completed in ${stats.timeSec.toFixed(2)}s.`));
				// TODO: more stats
				logger.info(c.green(`🚀 Processed ${stats.pages} pages.`));
			},
		},
	};
};
