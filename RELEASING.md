# Releasing jevrs

1. Move the `[Unreleased]` changelog entries under a new
   `## [x.y.z] - YYYY-MM-DD` heading.
2. Bump `version` in the root `Cargo.toml` `[workspace.package]` table and the
   `version = "..."` requirements for the `jevrs-core` and `jevrs-derive`
   dependencies in `crates/jevrs/Cargo.toml`. Then update the lockfile:

   ```console
   cargo update --workspace
   ```

3. Commit and tag the release, then push both:

   ```console
   git commit -am "release: x.y.z"
   git tag -a vx.y.z -m "jevrs x.y.z"
   git push origin main vx.y.z
   ```

4. The `release.yml` workflow verifies the release, publishes all three crates
   through trusted publishing, and creates the GitHub release from the
   changelog.

If publishing fails after some crates upload, a re-run cannot upload them
again. Publish the remaining crates by hand in dependency order (`jevrs-core`,
`jevrs-derive`, then `jevrs`) with
`nix develop -c cargo publish -p <crate> --locked`, then create the release
with `gh release create`.

## One-time setup

On crates.io, configure a trusted publisher for `jevrs-core`, `jevrs-derive`,
and `jevrs` with owner `luizribeiro`, repository `jevrs`, workflow
`release.yml`, and environment `release`.

GitHub creates the `release` environment on first use without protection. To
require a manual gate before publishing, add required reviewers under the
repository's Environments settings.
