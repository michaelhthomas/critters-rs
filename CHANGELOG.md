# Changelog

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
