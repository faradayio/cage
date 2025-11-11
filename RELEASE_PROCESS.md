# Release process

This document describes how to create a new release of cage.

## Prerequisites

- You must have push access to the repository
- You must be able to push tags to GitHub

## Steps to create a release

### 1. Update the version in Cargo.toml

Edit `Cargo.toml` and update the version number:

```toml
[package]
name = "cage"
version = "0.5.0"  # Update this line
```

### 2. Update CHANGELOG.md

Add a new section for the version you're releasing. The format should be:

```markdown
## 0.5.0 - YYYY-MM-DD

### Added
- New features go here

### Changed
- Changes to existing functionality

### Fixed
- Bug fixes
```

Make sure to:
- Move items from the `## Unreleased` section to your new version section
- Use today's date in YYYY-MM-DD format
- Follow semantic versioning principles

### 3. Commit the version changes

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "Bump version to 0.5.0"
git push origin master
```

### 4. Create and push a git tag

The tag MUST match the version in Cargo.toml with a `v` prefix:

```bash
# For version 0.5.0 in Cargo.toml, create tag v0.5.0
git tag v0.5.0
git push origin v0.5.0
```

**CRITICAL**: The tag version (without the `v` prefix) must exactly match the version in `Cargo.toml`. If they don't match, the GitHub Actions workflow will fail.

### 5. Wait for the automated build

Once you push the tag, GitHub Actions will automatically:

1. **Validate** that the tag version matches Cargo.toml
2. **Build** binaries for all platforms:
   - Linux (x86_64, statically linked with musl)
   - macOS Intel (x86_64)
   - macOS Apple Silicon (aarch64)
   - Windows (x86_64)
3. **Create** a GitHub release with all binaries attached
4. **Extract** the changelog section for this version and add it to the release notes

You can monitor the progress at: https://github.com/faradayio/cage/actions

The build typically takes 5-10 minutes.

### 6. Verify the release

Once the workflow completes:

1. Go to https://github.com/faradayio/cage/releases
2. Verify the new release appears with the correct version
3. Check that all 4 binary archives are attached:
   - `cage-v0.5.0-linux-x86_64.zip`
   - `cage-v0.5.0-macos-x86_64.zip`
   - `cage-v0.5.0-macos-aarch64.zip`
   - `cage-v0.5.0-windows-x86_64.zip`
4. Verify the changelog appears in the release description
5. Optionally, download and test a binary to ensure it works

## What if something goes wrong?

### The workflow fails at validation

**Error**: "Tag version (X.X.X) does not match Cargo.toml version (Y.Y.Y)"

**Fix**:
1. Delete the tag locally: `git tag -d vX.X.X`
2. Delete the tag remotely: `git push origin :refs/tags/vX.X.X`
3. Fix the version in `Cargo.toml` to match your intended version
4. Commit and push the fix
5. Create the correct tag and push it again

### The workflow fails during build

**Fix**:
1. Check the Actions tab for the specific error
2. If it's a build error, fix the code issue
3. Delete the tag (see above)
4. Increment the version number (e.g., 0.5.0 → 0.5.1)
5. Update both `Cargo.toml` and `CHANGELOG.md`
6. Commit, push, and create a new tag

### You need to update a release

If you need to add or replace binaries:
1. Go to the release page on GitHub
2. Click "Edit release"
3. You can upload new files or delete existing ones
4. Click "Update release"

### You tagged the wrong commit

**Fix**:
1. Delete the tag locally: `git tag -d vX.X.X`
2. Delete the tag remotely: `git push origin :refs/tags/vX.X.X`
3. Check out the correct commit: `git checkout <correct-commit-sha>`
4. Create the tag: `git tag vX.X.X`
5. Push the tag: `git push origin vX.X.X`

Note: If the automated release was already created, you'll need to delete it from GitHub's releases page first.

## Version numbering

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** version (X.0.0): Incompatible API changes
- **MINOR** version (0.X.0): New functionality in a backward compatible manner
- **PATCH** version (0.0.X): Backward compatible bug fixes

## Publishing to crates.io

The automated workflow does NOT publish to crates.io. To publish to crates.io:

```bash
cargo publish
```

You'll need appropriate credentials configured for crates.io.

