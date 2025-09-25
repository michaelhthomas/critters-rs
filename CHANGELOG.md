# Changelog

## [1.2.0](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v1.1.3...critters-rs-v1.2.0) (2025-09-25)


### Features

* add exclude_external option ([#14](https://github.com/michaelhthomas/critters-rs/issues/14)) ([8ceb30e](https://github.com/michaelhthomas/critters-rs/commit/8ceb30eadc0413f6d5789dc31612e4fb3ae586f7))


### Bug Fixes

* avoid integer overflow from szudzik pairing on large inputs ([4f7827a](https://github.com/michaelhthomas/critters-rs/commit/4f7827a37c6f224473544b72d796e1e811a6d681))
* trim slashes when deserializing regular expression matchers ([8ae8668](https://github.com/michaelhthomas/critters-rs/commit/8ae86687ce74f801babe3e01c135339057cba841))


### Performance Improvements

* optimized filtering of selector list ([54355e4](https://github.com/michaelhthomas/critters-rs/commit/54355e4ee7e0618e2e7ffb1059e6ecce933b58fe))
* store class list separately from other attributes ([72d815b](https://github.com/michaelhthomas/critters-rs/commit/72d815b595ebbd4f23fabce7acc3dbb463b37ae4))
* **style_calculation:** precompute "id" local name in hot path ([dcdbe73](https://github.com/michaelhthomas/critters-rs/commit/dcdbe73a9565845754846784ded2f4bf373c289c))

## [1.1.3](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v1.1.2...critters-rs-v1.1.3) (2025-06-15)


### Performance Improvements

* bump kichikiki to 0.8.3 ([45f90d9](https://github.com/michaelhthomas/critters-rs/commit/45f90d93da79aa63fd083e5d6f2f2915792db056))

## [1.1.2](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v1.1.1...critters-rs-v1.1.2) (2025-03-11)


### Bug Fixes

* use fork of kuchikiki to allow publishing on crates.io ([5cafa0a](https://github.com/michaelhthomas/critters-rs/commit/5cafa0a599de6f7836101d54b64fa01b92ad181a))

## [1.1.1](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v1.1.0...critters-rs-v1.1.1) (2025-01-09)


### Bug Fixes

* resolve upstream bug causing template contents to be erased ([2ab0702](https://github.com/michaelhthomas/critters-rs/commit/2ab0702178ca6b9bfa5c4afe1874c510dcd82b5f))

## [1.1.0](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v1.1.0...critters-rs-v1.1.0) (2024-12-31)


### Miscellaneous Chores

* **critters-rs:** Synchronize critters versions

## [1.1.0](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v1.0.3...critters-rs-v1.1.0) (2024-12-31)


### Features

* add support for allow_rules option ([2085113](https://github.com/michaelhthomas/critters-rs/commit/20851131776909c610e0fe740486e053a500d550))
* support critters container attribute ([14bc53e](https://github.com/michaelhthomas/critters-rs/commit/14bc53e50a8e5a2d8854f9847e2b75dbd3e5ef62))


### Bug Fixes

* change level of stylesheet resolution issues to warning ([9bcbd3e](https://github.com/michaelhthomas/critters-rs/commit/9bcbd3eb648de7482150b9f77e71dc4d254f65a5))
* properly resolve relative paths on POSIX platforms ([8074771](https://github.com/michaelhthomas/critters-rs/commit/807477199aac257ffda9968c26a60048b8b7fd62))


### Performance Improvements

* cache regular expressions ([5971da0](https://github.com/michaelhthomas/critters-rs/commit/5971da093188f6b8d174fe2929cffe88d3079e90))
* only take first element from selector match iterator ([40aac0a](https://github.com/michaelhthomas/critters-rs/commit/40aac0ab7759c7718726be18703ee91f42bbf4d8))

## [1.0.3](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v1.0.2...critters-rs-v1.0.3) (2024-12-30)


### Bug Fixes

* do not abort selector filtering after the first match ([eb4412b](https://github.com/michaelhthomas/critters-rs/commit/eb4412bcbd5b42fbb401d389b2ae1f48f1d81389))

## [1.0.2](https://github.com/michaelhthomas/critters-rs/compare/critters-rs-v0.1.0...critters-rs-v1.0.2) (2024-11-28)


### Features

* implement "media" preload strategy ([a4c5d06](https://github.com/michaelhthomas/critters-rs/commit/a4c5d063cd7c3bcb1992c3f038f996ccdb471d4b))
* implement "swap-high" preload strategy ([7086983](https://github.com/michaelhthomas/critters-rs/commit/7086983165ff3a49544c8c028193b0832652dc36))
* implement "swap" preload strategy ([07c7579](https://github.com/michaelhthomas/critters-rs/commit/07c75796a387f82d6269c91d75ceac2b8179b058))
* support merging stylesheets ([1588b9c](https://github.com/michaelhthomas/critters-rs/commit/1588b9c81dbf499ade82348febb7561e09af8ebf))


### Bug Fixes

* disable unimplemented preload strategies ([ba86c9d](https://github.com/michaelhthomas/critters-rs/commit/ba86c9d59bf5813c3f73d65c1d3385c95766efa3))

## 0.1.0 (2024-11-15)


### Features

* **@critters-rs/critters:** add basic tests ([bf4f1f3](https://github.com/michaelhthomas/critters-rs/commit/bf4f1f330dfe39f3a31e55385aaa2590f021eeea))
* add astro integration ([ed7f770](https://github.com/michaelhthomas/critters-rs/commit/ed7f770ce4f3130b1eb791466d0a5b41b9181622))
* add basic CLI ([1ce8a4e](https://github.com/michaelhthomas/critters-rs/commit/1ce8a4e3d3de5f1b352310773f35d05861180e31))
* add biome for formatting/linting ([df2f175](https://github.com/michaelhthomas/critters-rs/commit/df2f1754df12c1e6d671751b94600d372bc61768))
* add README.md ([7ea48ed](https://github.com/michaelhthomas/critters-rs/commit/7ea48ed78fea0e6fb34a307d803744bab662745a))
* **astro:** add README.md ([d9f5eda](https://github.com/michaelhthomas/critters-rs/commit/d9f5eda600d5967bddcacac9f4896d14f57c3ff6))
* **cli:** add logging support ([49c14b2](https://github.com/michaelhthomas/critters-rs/commit/49c14b2c9075b6b4a1c8e5dc4a075c3183992c9f))
* **cli:** add progress bar ([ff7d479](https://github.com/michaelhthomas/critters-rs/commit/ff7d47968cc943e6aba28a8945e186118eb7f46e))
* initial commit ([6bee6fe](https://github.com/michaelhthomas/critters-rs/commit/6bee6fe889ba7a836f7e3dffa8b72cd7858f5ecc))
* support building to WASM ([add2ec1](https://github.com/michaelhthomas/critters-rs/commit/add2ec10e688dd43e07312956659e40e9693226c))
* support external stylesheets ([d4525a1](https://github.com/michaelhthomas/critters-rs/commit/d4525a1ae2c4473ebf8cad578ddaf54afab5521f))
* update options to match critters ([57873b3](https://github.com/michaelhthomas/critters-rs/commit/57873b3c8e30b66c0328ffc5afebbdcf069fb6fc))


### Bug Fixes

* add artifacts command ([f803ded](https://github.com/michaelhthomas/critters-rs/commit/f803dedc0de2b41fba50cb739591f6b324ab2c07))
* add create npm dirs command ([01cd933](https://github.com/michaelhthomas/critters-rs/commit/01cd93357f3b0f7f4fe44e2b5d78794aca812a43))
* add packaging information to Cargo.toml ([22ef2cb](https://github.com/michaelhthomas/critters-rs/commit/22ef2cbbeb3068fcf1176e9ea3dc03b1015b1649))
* **build:** run cargo using `childProcess.spawn` ([bd95f6a](https://github.com/michaelhthomas/critters-rs/commit/bd95f6a108ca95a63fa973e9dd4fb54db9310f77))
* correctly inject font preload links ([d844640](https://github.com/michaelhthomas/critters-rs/commit/d844640c03c7eec58ee99c59add4e6c4b0ce6432))
* **critters-rs:** remove superfluous slashes from regular expressions ([ede2678](https://github.com/michaelhthomas/critters-rs/commit/ede2678a78ce581341f6ae6b24b9a4ea8e690570))
* **critters/napi:** enable logging ([275daaf](https://github.com/michaelhthomas/critters-rs/commit/275daafa2eb82c5b579e59b810a155e8b2a672ad))
* improved output path warning ([35acadc](https://github.com/michaelhthomas/critters-rs/commit/35acadc8820fb0d308a2349f0a9aca02b537f2e0))
* **utils:** improve pairing function to avoid overflows ([ac13deb](https://github.com/michaelhthomas/critters-rs/commit/ac13deb211d268f0a364b0b03303aef6d8811283))


### Performance Improvements

* parallelize processing of HTML files ([a10389c](https://github.com/michaelhthomas/critters-rs/commit/a10389ce5a13dd89fe6e5b790051524bad7b3338))
