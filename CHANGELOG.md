# Changelog

## [0.3.0](https://github.com/blackjid/fantom-librarian/compare/v0.2.0...v0.3.0) (2026-11-09)


### Features

* **cli:** scene extract, canary and merge carry sequencer patterns ([17037a2](https://github.com/blackjid/fantom-librarian/commit/17037a250f8ece6ef3b22fa5fa8af89717ed1ecf))
* **core:** a requirement answers met, not loaded, or missing ([71972b4](https://github.com/blackjid/fantom-librarian/commit/71972b4e2f0ac5a8fee4e9e30b3283580c316bdc))
* **core:** blank the origin stamp on every rebuild ([80bbf79](https://github.com/blackjid/fantom-librarian/commit/80bbf79c173bf6b03ab59f3b27f94eee42d3c66b))
* **core:** DIFa is an origin stamp, not a checksum ([3fc72c2](https://github.com/blackjid/fantom-librarian/commit/3fc72c2fd98802b54414fce5c3daee192c3db33c))


### Fixes

* **core:** a mono sample keeps its channel count in an SVZ ([9caaaa0](https://github.com/blackjid/fantom-librarian/commit/9caaaa0f165a9e287fc8fbc2ee69c2e3a03ea9f5))
* **core:** sound names as the instrument spells them ([cae983c](https://github.com/blackjid/fantom-librarian/commit/cae983c2442ed6cc311b6cb42c28b4d5a1f2b220))
* **desktop:** a factory preset is not a warning ([3f29454](https://github.com/blackjid/fantom-librarian/commit/3f29454127df432a7cfad8b82689a87860bcb03a))
* **library:** stop seeding the factory scenes ([65612d2](https://github.com/blackjid/fantom-librarian/commit/65612d2af5774ce35d13cced0b25977a5e65e7bc))


### Changes

* **library:** one ladder instead of owned and loaded ([723fbaf](https://github.com/blackjid/fantom-librarian/commit/723fbaf6f2a51796aec7fe0fa3faab9998524a0d))


### Documentation

* ACB model IDs and tone names from ACBa records ([b3db5dc](https://github.com/blackjid/fantom-librarian/commit/b3db5dc37005a149358429aebdcf0a6d1f44dd11))
* ACBa tone layout — 2419 named parameters ([24a68f1](https://github.com/blackjid/fantom-librarian/commit/24a68f162397c8429307fc5a514313042d9fecaa))
* **core:** the DIFa token is the instrument's registration ID ([c802c60](https://github.com/blackjid/fantom-librarian/commit/c802c605860f5dac255bd3929a0a2adf899e5df0))
* decode SeqSet.SQS patterns and events ([0a58085](https://github.com/blackjid/fantom-librarian/commit/0a5808541507a497f9e99124b25d6a73d8473f46))
* decode SYSa system settings; the SVZ stamp byte is DEVELOP_VER ([3904e08](https://github.com/blackjid/fantom-librarian/commit/3904e08a9aa114b09fd15c8d348226970c08398e))
* Lock bit isn't only factory samples ([392ff8c](https://github.com/blackjid/fantom-librarian/commit/392ff8c806b54cbf6c259dc90514d6bbffedb10d))
* MDLa JD-800 and WT block placement ([d591fc1](https://github.com/blackjid/fantom-librarian/commit/d591fc1eb597e5e7cbba43f0553949733d262591))
* MDLa MODEL tone layout, types and parameters ([db5597e](https://github.com/blackjid/fantom-librarian/commit/db5597e792124295adf732db0d1e0ec0f96c653c))
* pad sampler — PADCONF.BIN and .SMP ([b616bba](https://github.com/blackjid/fantom-librarian/commit/b616bba2e6888feba443809e0f16dddc26d2de8c))
* PRFa header's info_length, ACBa slots ([6b928ff](https://github.com/blackjid/fantom-librarian/commit/6b928ffb82198d8b32ade9f726344ca607c4edfd))
* **readme:** scene commands carry SEQ/ ([0bf152d](https://github.com/blackjid/fantom-librarian/commit/0bf152d043317590fe4927ee715a8d08b51a18b9))
* record sizes, slot counts and import checks from the instrument ([a069775](https://github.com/blackjid/fantom-librarian/commit/a0697751b98dd1b5006f84c34abfb9465ccf7eeb))
* SMPd Lock bit and Hash-Tag ([815fd2c](https://github.com/blackjid/fantom-librarian/commit/815fd2ca17d43459d54c4896b4a7eaa1ab048535))
* SQS event kinds and automation lanes; delta follows the event ([e5136eb](https://github.com/blackjid/fantom-librarian/commit/e5136eb4a330a38eaa86efa7555a6c4cff4766d9))
* SQS pattern table only holds for an empty set ([d46298b](https://github.com/blackjid/fantom-librarian/commit/d46298b219fe811bcad8b7263eb5861dee730d8c))
* SVZ import ignores the model stamp ([c898f58](https://github.com/blackjid/fantom-librarian/commit/c898f58887e71001b7d149783fa5d60b40a41a6f))
* SYSa Input Reverb per-type parameters; pad bytes reserved ([8a0867e](https://github.com/blackjid/fantom-librarian/commit/8a0867e599ec44657c010a14afb631ea60312cd0))
* the EX manuals sit in the private corpus ([8fef388](https://github.com/blackjid/fantom-librarian/commit/8fef3888f2525419115648f7676f28ba71b889d3))
* the files beside FANTOM.SVD — SQS, chains, ratings, pad samples ([25f9500](https://github.com/blackjid/fantom-librarian/commit/25f95000303f2bb62ddf74f575fd5ad244f57b88))
* the SVZ preamble stamp is the model code plus SYSa[0xfb] ([8be6a60](https://github.com/blackjid/fantom-librarian/commit/8be6a606e9f8f3aef9c865587e1ed36ffcdfcd8d))
* wallpaper XML; Roland's scene packs use the export layout ([56a3ef5](https://github.com/blackjid/fantom-librarian/commit/56a3ef5f75fdc4b789a01893f92519297664178c))
* where the instrument's DIFa comes from ([396173a](https://github.com/blackjid/fantom-librarian/commit/396173acdae7edac218252a8cc063cd0c7b6ca90))

## [0.2.0](https://github.com/blackjid/fantom-librarian/compare/v0.1.0...v0.2.0) (2026-09-02)


### Features

* **desktop:** a personal installation you can upgrade ([9576eea](https://github.com/blackjid/fantom-librarian/commit/9576eeab5751c3253c40d3c266c8a21e81a643ba))
