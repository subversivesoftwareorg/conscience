# Publishing Conscience

## Install Methods (for users)

### From GitHub (works now)

```bash
cargo install --git https://github.com/subversivesoftwareorg/conscience
```

### From crates.io (after publishing)

```bash
cargo install conscience
```

### From a release binary (after tagging)

Download from the [Releases page](https://github.com/subversivesoftwareorg/conscience/releases) and place in your PATH.

## Publishing to crates.io

### Prerequisites

1. Create an account at [crates.io](https://crates.io/) (login with GitHub)
2. Generate an API token at [crates.io/settings/tokens](https://crates.io/settings/tokens)
3. Login from your terminal:
   ```bash
   cargo login <your-token>
   ```

### Pre-publish Checklist

1. **Check the crate name is available:**
   ```bash
   cargo search conscience
   ```
   If taken, update `name` in `Cargo.toml`.

2. **Ensure required fields in `Cargo.toml`:**
   ```toml
   [package]
   name = "conscience"
   version = "0.1.0"
   edition = "2021"
   description = "Evaluate the impact of AI-assisted work"
   license = "MIT"              # or your chosen license
   repository = "https://github.com/subversivesoftwareorg/conscience"
   homepage = "https://github.com/subversivesoftwareorg/conscience"
   keywords = ["ai", "ethics", "analysis", "cli"]
   categories = ["command-line-utilities", "development-tools"]
   ```

3. **Add a LICENSE file** (currently TBD in README):
   ```bash
   # For MIT:
   curl -sL https://opensource.org/licenses/MIT > LICENSE
   # Edit to add your name and year
   ```

4. **Verify the package builds and looks right:**
   ```bash
   cargo package --list        # see what will be published
   cargo package               # build the package locally
   ```

5. **Check that `refs/` PDFs are excluded** (they're large and shouldn't go to crates.io). Add to `Cargo.toml` if needed:
   ```toml
   [package]
   exclude = ["refs/"]
   ```

### Publish

```bash
cargo publish
```

### After Publishing

Users can install with:
```bash
cargo install conscience
```

Update the conscience.yml workflow install step to:
```yaml
- name: Install Conscience
  run: cargo install conscience
```

## Creating a Release (GitHub)

Tag the commit and push:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers `.github/workflows/release.yml` which:
1. Builds binaries for Linux (amd64, arm64) and macOS (amd64, arm64)
2. Creates a GitHub Release with all binaries and SHA256 checksums
3. Auto-generates release notes from commits since the last tag

## Version Bumping

1. Update `version` in `Cargo.toml`
2. Commit: `git commit -am "Bump version to 0.2.0"`
3. Tag: `git tag v0.2.0`
4. Push: `git push origin main v0.2.0`
5. Publish: `cargo publish`
