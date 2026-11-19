# The updater's test key pair

`update.key` (the 64-byte private key) and `update.pub` (the 32-byte
verifying key) are a zipsign 0.2.2 ed25519 key pair made with `zipsign
gen-key` for test builds only. The private key is public on purpose: anyone
can sign with it.

**No released build trusts this key.** A `Package` run dispatched with
`update_test_repo` signs the updater's archives with `update.key`, and only a
build made with `HANDPIC_UPDATE_TEST_REPO` set (the same run's) verifies with
`update.pub` and reads that scratch repository instead of the real releases.
Such a build must never be distributed.

Released builds verify with the production key, `../update.pub`, whose
private key the maintainer keeps outside the repository
(`mise run app:keygen`).
