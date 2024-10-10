# `@critters-rs/astro`

Rapidly inline your Astro site's critical CSS with [`critters-rs`](https://github.com/michaelhthomas/critters-rs).

This Astro integration operates on the generated output for the static pages on your site, inlining the critical CSS within each HTML file while deferring unused styles to be loaded after the page has painted. This should help to reduce delays in FCP and LCP caused by stylesheet loading and parsing.

## Installation

### Using `astro add`

The easiest way to install `@critters-rs/astro` is the `astro add` command, which automatically installs this package as a dependency and adds it to your Astro configuration. To use it, run the following command and follow the prompts:

Using NPM:

```sh
npx astro add @critters-rs/astro
```

Using Yarn:

```sh
yarn astro add @critters-rs/astro
```

Using PNPM:

```sh
pnpm astro add @critters-rs/astro
```

### Manual Installation

First, install the integration package using your preferred package manager.

```sh
pnpm add -D @critters-rs/astro
```

Then, add the integration to the `integrations` section of `astro.config.*`.

```ts
import critters from '@critters-rs/astro';

export default defineConfig({
	integrations: [critters()],
});
```

## Usage

For simple use-cases, the above setup should be enough to see performance improvements, and *should* not affect the appearance of any pages. That said, the `critters()` function takes an optional set of options to configure the behavior of `critters`. [The full set of available options can be seen here](https://docs.rs/critters-rs/latest/critters_rs/struct.CrittersOptions.html), and are also available in the typescript definitions published with the package.

```ts
import critters from '@critters-rs/astro';

export default defineConfig({
  integrations: [
    critters({
      publicPath: 'https://example.com',
      external: true,
      mergeStylesheets: true,
      ...
    })
  ],
});
```
