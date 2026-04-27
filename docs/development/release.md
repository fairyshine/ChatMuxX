# Release

Status: draft.

ChatMuxX uses the Rust workspace version as the single source of truth.

## Version Source

The version is defined once in the root `Cargo.toml`:

```toml
[workspace.package]
version = "0.0.1"
```

Workspace crates inherit it with:

```toml
version.workspace = true
```

Do not edit crate versions separately.

Prerelease versions are supported. For example, `0.0.1-dev` is valid and should be tagged as `v0.0.1-dev`.

## Release Flow

1. Update `workspace.package.version` in the root `Cargo.toml`.
2. Run the local checks:

   ```bash
   cargo fmt --check
   cargo test --locked
   cargo clippy --all-targets --locked -- -D warnings
   cargo build --locked
   ```

3. Commit the version change.
4. Create and push a matching tag:

   ```bash
   git tag v0.0.1
   git push origin v0.0.1
   ```

The tag must match the Cargo version exactly after removing the leading `v`.
For example, tag `v0.0.1` requires Cargo version `0.0.1`.
Tag `v0.0.1-dev` requires Cargo version `0.0.1-dev`.

## GitHub Actions

- `.github/workflows/ci.yml` runs formatting, tests, and clippy on `master` and pull requests.
- `.github/workflows/release.yml` runs when a `v*` tag is pushed, then verifies that the tag matches the Cargo version.
- `.github/workflows/release.yml` can also be run manually with `workflow_dispatch` for an existing tag, such as `v0.0.1-dev1`, to rebuild and upload release assets.
- Versions containing `-`, such as `0.0.1-dev`, are published as GitHub prereleases.
- The release workflow builds `cmx` archives for Linux x86_64, Linux aarch64, macOS x86_64, macOS arm64, Windows x86_64, and Windows aarch64.
- The release workflow uploads `.tar.gz` archives and `.sha256` checksum files to the GitHub release.
- `scripts/install.sh` installs from GitHub Releases by default. The default `CHATMUXX_VERSION=latest-prerelease` includes pre-releases. Use `CHATMUXX_VERSION=latest-stable` for the latest stable release, or set an exact tag such as `CHATMUXX_VERSION=v0.0.1-dev1`.
- Set `CHATMUXX_INSTALL_METHOD=source` to force source install with git and Rust/Cargo.
