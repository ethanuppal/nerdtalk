# Changelog

## [0.2.0](https://github.com/ethanuppal/nerdtalk/compare/v0.1.0...v0.2.0) (2025-01-13)


### Features

* **protocol:** Tag server entries reply with client id ([#53](https://github.com/ethanuppal/nerdtalk/issues/53)) ([8a87638](https://github.com/ethanuppal/nerdtalk/commit/8a8763890db986d461a3ad2a57b9f84a071711f4))


### Miscellaneous Chores

* Add LICENSE ([#54](https://github.com/ethanuppal/nerdtalk/issues/54)) ([6f3210b](https://github.com/ethanuppal/nerdtalk/commit/6f3210bb18efce4dd8911f49515ea7c4988e82cb))
* **lints:** Add more abbreviation lints ([#55](https://github.com/ethanuppal/nerdtalk/issues/55)) ([a5afc56](https://github.com/ethanuppal/nerdtalk/commit/a5afc56ebd3c557ee7f3e394ba4e405c75437609))
* **protocol:** Use JSON over bincode for passing data over websockets ([#51](https://github.com/ethanuppal/nerdtalk/issues/51)) ([102f446](https://github.com/ethanuppal/nerdtalk/commit/102f446103625e2a902fbad21544437ffe288ab9))

## 0.1.0 (2025-01-01)


### Features

* Add TUI ([#7](https://github.com/ethanuppal/nerdtalk/issues/7)) ([78910d7](https://github.com/ethanuppal/nerdtalk/commit/78910d74395bce57db67845448056b2dfc4c9b7a))
* **protocol:** Implement entry range request ([#40](https://github.com/ethanuppal/nerdtalk/issues/40)) ([678eeeb](https://github.com/ethanuppal/nerdtalk/commit/678eeeb28ae713cc2bbaef0fde5e49e8571642f0))
* Secure websockets over TLS ([#3](https://github.com/ethanuppal/nerdtalk/issues/3)) ([21a1d03](https://github.com/ethanuppal/nerdtalk/commit/21a1d03a944b87ebf2618eba185f6ad878320d08))
* Start work on message sending ([#4](https://github.com/ethanuppal/nerdtalk/issues/4)) ([c1183d0](https://github.com/ethanuppal/nerdtalk/commit/c1183d0dc0c3cdd2aa6b7ec1ad149b4af3592bbd))
* **tui:** Integrate protocol into TUI ([#47](https://github.com/ethanuppal/nerdtalk/issues/47)) ([6b82ccd](https://github.com/ethanuppal/nerdtalk/commit/6b82ccd46377db849a0d3434320619d077b540f8))
* Update web app with core UI scaffold ([#11](https://github.com/ethanuppal/nerdtalk/issues/11)) ([f684760](https://github.com/ethanuppal/nerdtalk/commit/f68476045b3dca5ec8bb3bc1ab22359f51f813f0))


### Bug Fixes

* Fix CI badges ([#2](https://github.com/ethanuppal/nerdtalk/issues/2)) ([6df50e2](https://github.com/ethanuppal/nerdtalk/commit/6df50e2e334ffb7230555add1442c13793ed7856))
* Release PRs now opened by github-actions[bot] ([#25](https://github.com/ethanuppal/nerdtalk/issues/25)) ([f3949ab](https://github.com/ethanuppal/nerdtalk/commit/f3949abedc7efa4e3f5be160fd274579e2a9adc4))
* **xtask-lint:** Handle constants correctly ([#48](https://github.com/ethanuppal/nerdtalk/issues/48)) ([93c3b67](https://github.com/ethanuppal/nerdtalk/commit/93c3b675c32be7cf286a8876e539c858c79dd2ec))


### Performance Improvements

* **tui:** Use `try_write` to prevent round-robin blocking in message updates ([#31](https://github.com/ethanuppal/nerdtalk/issues/31)) ([23e9f06](https://github.com/ethanuppal/nerdtalk/commit/23e9f06c851e0f940209410ba3b7c36d8a6528db))


### Reverts

* **ci:** Run on macOS again ([#34](https://github.com/ethanuppal/nerdtalk/issues/34)) ([b8113f5](https://github.com/ethanuppal/nerdtalk/commit/b8113f55a8a777e9a0d4294dee7019b8ab8d7d13))


### Miscellaneous Chores

* Setup release-please ([#17](https://github.com/ethanuppal/nerdtalk/issues/17)) ([53bcd2c](https://github.com/ethanuppal/nerdtalk/commit/53bcd2c3070b678d5c38a7f87f6ba429b128c709))
* Test autorelease ([#23](https://github.com/ethanuppal/nerdtalk/issues/23)) ([30e82f5](https://github.com/ethanuppal/nerdtalk/commit/30e82f538bb3ab94bafa523a681770e937642c88))


### Code Refactoring

* **chat, server:** Consistent server logging ([#44](https://github.com/ethanuppal/nerdtalk/issues/44)) ([f5d1be9](https://github.com/ethanuppal/nerdtalk/commit/f5d1be97a9cf9d40e3e5c3ee0b334f8895ec3fd8))
* **chat:** Richer chat entry representation ([#35](https://github.com/ethanuppal/nerdtalk/issues/35)) ([e57e668](https://github.com/ethanuppal/nerdtalk/commit/e57e668a7646718abc6919ad1d4fdb2de366a42f))
* **scripts:** Move testing scripts to designated folder ([#29](https://github.com/ethanuppal/nerdtalk/issues/29)) ([99d1cc1](https://github.com/ethanuppal/nerdtalk/commit/99d1cc1e9841986eea1ae990adf0b8a4258c4452))
* Streamline client connection library and server ([#6](https://github.com/ethanuppal/nerdtalk/issues/6)) ([fc2491f](https://github.com/ethanuppal/nerdtalk/commit/fc2491f67b454490a51a1915194cc713c933f8d5))
* Vim parsing and command application ([#9](https://github.com/ethanuppal/nerdtalk/issues/9)) ([2073a72](https://github.com/ethanuppal/nerdtalk/commit/2073a729e787b22c75e4016a1a1f6d18daf4fc81))


### Continuous Integration

* **lint:** Add abbreviation checker ([#45](https://github.com/ethanuppal/nerdtalk/issues/45)) ([ace8607](https://github.com/ethanuppal/nerdtalk/commit/ace8607451eb7f225b29feb2200981891eb1c1e3))
* **lint:** Better PR title linting ([#28](https://github.com/ethanuppal/nerdtalk/issues/28)) ([7d54afa](https://github.com/ethanuppal/nerdtalk/commit/7d54afa35cb21bb950ef4d89d912e5c42dc9119f))
* **lint:** Install nightly `rustfmt` ([#39](https://github.com/ethanuppal/nerdtalk/issues/39)) ([340a71e](https://github.com/ethanuppal/nerdtalk/commit/340a71e862fb350b2ea21fde8a73f96387754da4))
* **perf:** Don't use incremental builds ([#49](https://github.com/ethanuppal/nerdtalk/issues/49)) ([22b4194](https://github.com/ethanuppal/nerdtalk/commit/22b41949a47eb5f51a5c0cf9ce1f5f57288a29d6))
* **perf:** Get rid of macOS runners altogether ([#32](https://github.com/ethanuppal/nerdtalk/issues/32)) ([21b3641](https://github.com/ethanuppal/nerdtalk/commit/21b364109bfb70c5e6821ce5b5ae8a98a32ca0c7))
* **perf:** Remove redundant effort ([#30](https://github.com/ethanuppal/nerdtalk/issues/30)) ([e6d867d](https://github.com/ethanuppal/nerdtalk/commit/e6d867dd0190b63f1fe685692aa48db2c021aeee))
* Random devops stuff ([#1](https://github.com/ethanuppal/nerdtalk/issues/1)) ([3ec0925](https://github.com/ethanuppal/nerdtalk/commit/3ec09255507146084dbe1da154929f9092fb1d45))
