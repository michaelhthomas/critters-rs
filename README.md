<div align="center">
	<h1><code>critters_rs</code></h1>
</div>

Rapidly inline your site's [critical CSS].

## Features

- Extremely fast -- written in pure Rust for maximal performance
- Robust -- uses HTML and CSS parsers from the [Servo](https://servo.org/) project
- Integrations with popular frameworks
- Supports preloading and/or inlining critical fonts
- Prunes unused CSS keyframes and media queries
- Minifies CSS using Parcel's `lightningcss`
- Supports pruning inlined CSS rules from lazy-loaded stylesheets

## Usage

### CLI

First, install the `critters` cli command globally:

```sh
cargo install critters-rs
```

Then, execute `critters` on a folder with your preferred options,

```sh
critters -p ./dist {options}
```

or view all the possible options:

```sh
critters -h
```

### NodeJS API

See the documentation in the [package's README](./packages/critters/README.md).

### Rust Crate

View the documentation for the Rust crate at https://docs.rs/critters-rs/.

### Other Integrations

View the documentation for other integrations in their package READMEs:

- [Astro](./packages/astro/README.md)

## Acknowledgements

This project is heavily inspired by https://github.com/GoogleChromeLabs/critters, aiming to provide identical functionality while offering considerably improved performance. Much credit goes to the Google Chrome team for their work on the original library.

## License

[Apache 2.0](LICENSE)

[critical css]: https://www.smashingmagazine.com/2015/08/understanding-critical-css/
