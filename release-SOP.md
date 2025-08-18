# Termal – Release Standard Operating Procedure (SOP)

 A short, repeatable checklist to publish a new Termal version across
 crates.io, GitHub, and Zenodo, including binary artifacts and citation
 metadata.

---

## 1) Choose the version & plan the release

* Follow **Semantic Versioning**: `MAJOR.MINOR.PATCH`.
* Decide **release type**:

  * **PATCH**: bug fixes, docs, no API/CLI breaking changes.
  * **MINOR**: backward-compatible features.
  * **MAJOR**: breaking changes (CLI flags/behavior, MSRV bump, file formats).

* Create or update a GitHub **milestone** for `vX.Y.Z` and make sure targeted issues/PRs are closed or moved.

---

## 2) Pre-flight checklist (local)

* [ ] Rust toolchain up-to-date and MSRV respected (document MSRV in `README.md`).
* [ ] All tests green locally and on `master`.
* [ ] Lints & formatting clean.
* [ ] Changelog updated.
* [ ] Docs (README, usage examples) current.
* [ ] License year & authorship current.
* [ ] Release artifacts include example file: `data/example-1.msa`.

Run locally:

```bash
# from repo root
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -D warnings
cargo test --all-features
```

---

## 3) Update metadata & version

* **Bump version** in `Cargo.toml`:

```toml
[package]
version = "X.Y.Z"
```

* **Sync crate metadata** (ensure these are accurate):

* `description`, `repository`, `documentation`, `license`, `readme`, `keywords`, `categories`.
* Make sure the crate includes needed files (see §8) via `include = [ ... ]` or `.cargo_vcs_info.json` (default ok if repo is clean).
* **CHANGELOG** (Keep a Changelog style): add a `## [X.Y.Z] - YYYY-MM-DD` section with highlights.
* **README**: make sure badges are still Ok (crates.io, docs.rs, CI), usage, install instructions, and **DOI badge** (concept DOI stays the same; see §10).
* **CITATION.cff**: bump `version` and `date-released` to `YYYY-MM-DD`; keep Zenodo **concept DOI**.

Verify package content before publishing:

```bash
cargo package --allow-dirty --no-verify  # creates .crate locally without publishing
cargo package --list | less              # inspect files to be uploaded
```

---

## 4) Build release artifacts locally (sanity check)

CI will produce the official binaries (see `./github/workflows/release.yaml`) ,
but a quick local build helps catch issues early.

```bash
# --lock makes sure `Cargo.lock` won't need any changes
cargo build --release --lock
```

Smoke test with the example alignment:

```bash
./target/release/termal data/example-1.msa --help
```

---

## 5) Tag the release & push

> CI is triggered by a **tag**. Commit all changes (version bump, changelog,
> docs), then tag.

```bash
git add -A
git commit -m "release: vX.Y.Z"
# Create an annotated (or signed) tag
git tag -a vX.Y.Z -m "termal vX.Y.Z"
# Merge the release branch if you use one
git checkout master && git merge --no-ff release/vX.Y.Z
# Push branch and tag
git push origin master
git push origin vX.Y.Z
```

This should automatically propagate to Zenodo, once the new release is on
GitHub. This is configured in Zenodo, not GitHub.


---

## 6) crates.io publish (source crate)

**NOTE** Crates.io tokens have a limited shelf life. In case of authorization
problems, it may be necessary to create a new token. Just log into Crates.io
(using te Github account), and create a new token. Also, do a `cargo logout`
then a `cargo login` with the new token.

> Publishing after pushing the tag ensures docs and links are consistent with the release.

Dry-run first:

```bash
cargo publish --dry-run
```

If OK, publish:

```bash
cargo publish
```

**Post-publish**: confirm on crates.io that `termal X.Y.Z` appears.

---

## 7) GitHub Release (binaries & notes)

* CI should run automatically on tag `vX.Y.Z`:

  * **Test workflow**: unit/integration tests.
  * **Release workflow**: builds artifacts for Linux/macOS/Windows, uploads to the GitHub Release.
* When the workflow finishes, create (or finalize) the **GitHub Release**:

  * Title: `termal vX.Y.Z`.
  * Release notes: paste from CHANGELOG `X.Y.Z`.
  * Ensure assets are attached (see §8) and SHAs are provided.

> If your workflow auto-creates the release and uploads assets, just verify everything below is present.

---

## 8) Required release assets (per platform)

Provide **compressed archives** that include:

**Common contents (inside each archive):**

* `termal` binary (or `termal.exe` on Windows)
* `README.md`, `LICENSE`, `CHANGELOG.md`
* `data/example-1.msa` (for quick testing)
* (Optional) `completions/` for bash/zsh/fish

**Recommended target set:**

* `x86_64-unknown-linux-gnu`
* `x86_64-unknown-linux-musl` (static)
* `aarch64-unknown-linux-gnu`
* `x86_64-apple-darwin` (Intel)
* `aarch64-apple-darwin` (Apple Silicon)
* `x86_64-pc-windows-msvc`

**File naming scheme:**

```
termal-vX.Y.Z-<target>.tar.gz    # unix
termal-vX.Y.Z-<target>.zip       # windows
```

**Checksums:**

* Upload a `SHA256SUMS.txt` covering all archives and a detached signature if you sign them.

> CI tip: Use a matrix job to build targets and a final job to collect artifacts, generate checksums, and upload to the release.

---

## 9) docs.rs & README links

* Ensure docs build on **docs.rs** (it runs automatically on publish):

  * If special features are needed for docs, set `package.metadata.docs.rs` in `Cargo.toml`.
* Verify README links and images; check badges display correctly on GitHub and crates.io.

---

## 10) Zenodo integration & citation

* Confirm **Zenodo–GitHub** integration is enabled for the repo and that your `.zenodo.json` (if any) and `CITATION.cff` are correct.
* After the GitHub Release is published, Zenodo should auto-archive it and mint a **version-specific DOI** tied to the **concept DOI**.
* **Action items:**

  * [ ] Visit Zenodo record for `vX.Y.Z` and verify metadata (title, authors, description, keywords).
  * [ ] Keep the **concept DOI** badge in `README.md` (unchanged across versions). Optionally add a note linking to latest version DOI.
  * [ ] If necessary, update `CITATION.cff` with `version` and `date-released` (already done in §3).

---

## 11) Post-release verification

* [ ] `cargo install termal` installs `X.Y.Z` on a clean machine/container.
* [ ] Each downloadable binary runs `--version` and opens `data/example-1.msa`.
* [ ] GitHub Release has all assets + checksums; CI badges green.
* [ ] crates.io page shows correct README and categories.
* [ ] docs.rs renders without warnings; examples compile.
* [ ] Zenodo archive created for the release; DOI visible.
* [ ] Open a new milestone `vX.Y.(Z+1)` and triage leftover issues.

---

## 12) Optional: automate with `cargo-release`

If you prefer a single command release, configure \[`cargo-release`] in `Release.toml`:

* Bump version, update changelog, tag, push, and even `cargo publish`.
* Ensure it cooperates with your CI (don’t double-publish).

Example minimal `Release.toml` (adapt as needed):

```toml
sign-tag = false
consolidate-commits = true
push = true
publish = false          # let CI publish, or set true if you publish locally
tag-message = "termal {{version}}"
pre-release-replacements = [
  { file = "CHANGELOG.md", search = "## \[Unreleased\]", replace = "## [Unreleased]\n\n## [{{version}}] - {{date}}" },
]
```

---

## 13) Rollback / recovery

* **Crate publish failed** after tag:

  * Fix issue → bump patch version → retag `vX.Y.(Z+1)` → publish.
* **Published crate has a critical bug**:

  * Yank the version on crates.io: `cargo yank --vers X.Y.Z`.
  * Release a hotfix `X.Y.(Z+1)` with clear notes in CHANGELOG.
* **Bad binary artifact**:

  * Replace assets on the GitHub Release (or delete and recreate) and regenerate checksums; announce fix.

---

## 14) One-page command recap

```bash
# Preflight
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -D warnings
cargo test --all-features

# Bump version in Cargo.toml, update CHANGELOG/README/CITATION.cff
cargo package --allow-dirty --no-verify
cargo package --list | less

# Tag & push
git checkout -b release/vX.Y.Z
git commit -am "release: vX.Y.Z"
git tag -a vX.Y.Z -m "termal vX.Y.Z"
git checkout master && git merge --no-ff release/vX.Y.Z
git push origin master
git push origin vX.Y.Z

# Publish crate
cargo publish --dry-run
cargo publish
# (CI builds binaries, creates release, uploads assets)
```

---

## 15) Checklists (copy/paste)

**Release readiness**

* [ ] Version chosen (SemVer) and milestone closed
* [ ] CHANGELOG updated
* [ ] README updated (badges, install, examples)
* [ ] CITATION.cff `version` & `date-released`
* [ ] License year updated
* [ ] Example file included in archives (`data/example-1.msa`)
* [ ] Tests, clippy, fmt, docs ok

**After tag/CI**

* [ ] GitHub Release exists with all artifacts & checksums
* [ ] crates.io shows `X.Y.Z`
* [ ] docs.rs built
* [ ] Zenodo archived & DOI visible
* [ ] `cargo install termal` works

---
