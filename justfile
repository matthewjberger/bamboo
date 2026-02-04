set windows-shell := ["powershell.exe"]

# Displays the list of available commands
@just:
    just --list

# Builds the project in release mode
build:
    cargo build -r

# Runs cargo check and format check
check:
    cargo check --all --tests
    cargo fmt --all -- --check

# Generates and opens documentation
docs:
    cargo doc --open -p bamboo

# Fixes linting issues automatically
fix:
    cargo clippy --all --tests --fix

# Formats the code using cargo fmt
format:
    cargo fmt --all

# Install development tools
install-tools:
    cargo install cargo-license
    cargo install cargo-about
    cargo install cargo-deny
    cargo install cargo-machete
    cargo install git-cliff

# Runs linter and displays warnings
lint:
    cargo clippy --all --tests -- -D warnings

# Publishes bamboo-ssg to crates.io (must be published before bamboo-cli)
publish-ssg:
    cargo publish -p bamboo-ssg

# Publishes bamboo-cli to crates.io (requires bamboo-ssg to be published first)
publish-cli:
    cargo publish -p bamboo-cli

# Publishes both crates to crates.io (bamboo-ssg first, then bamboo-cli)
publish-crate:
    cargo publish -p bamboo-ssg
    cargo publish -p bamboo-cli

# Dry run of publishing bamboo-ssg
publish-crate-dry:
    cargo publish -p bamboo-ssg --dry-run

# Runs the CLI
run *args:
    cargo run -r -p bamboo-cli -- {{args}}

# Runs an example site (blog, slideshow)
example name="blog":
    cargo run -r -p bamboo-cli -- serve --input examples/{{name}}

# Builds an example site
build-example name="blog":
    cargo run -r -p bamboo-cli -- build --input examples/{{name}} --output examples/{{name}}/dist

# Runs all tests
test:
    cargo test --all -- --nocapture

# Checks for unused dependencies
udeps:
    cargo machete

# Prints a table of all dependencies and their licenses
licenses:
    cargo license

# Checks for problematic licenses in dependencies
licenses-check:
    cargo deny check licenses

# Generates third-party license attribution document
licenses-html:
    cargo about generate about.hbs -o THIRD_PARTY_LICENSES.html

# Displays version information for Rust tools
@versions:
    rustc --version
    cargo fmt -- --version
    cargo clippy -- --version

# Watches for changes and runs tests
watch:
    cargo watch -x 'test --all'

# Generates changelog using git-cliff
changelog:
    git cliff -o CHANGELOG.md

# Shows the last tagged commit
show-tag:
    git describe --tags --abbrev=0

# Shows the current version from Cargo.toml (Windows)
[windows]
show-version:
    $version = (Select-String -Path 'crates/bamboo/Cargo.toml' -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value; Write-Host "v$version"

# Shows the current version from Cargo.toml (Unix)
[unix]
show-version:
    @echo "v$(grep '^version = ' crates/bamboo/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')"

# Deletes a git tag locally and remotely
strip-tag tag:
    git tag -d {{tag}}
    git push origin :refs/tags/{{tag}}

# Pushes a version tag and commits (Windows)
[windows]
push-version:
    $version = (Select-String -Path 'crates/bamboo/Cargo.toml' -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value; git push origin "v$version"; git push

# Pushes a version tag and commits (Unix)
[unix]
push-version:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$(grep '^version = ' crates/bamboo/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    git push origin "v$VERSION"
    git push

# Creates a GitHub release for the current version (Windows)
[windows]
publish-release:
    $version = (Select-String -Path 'crates/bamboo/Cargo.toml' -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value; git cliff --latest | gh release create "v$version" --title "bamboo-v$version" --notes-file -

# Creates a GitHub release for the current version (Unix)
[unix]
publish-release:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$(grep '^version = ' crates/bamboo/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    git cliff --latest | gh release create "v$VERSION" --title "bamboo-v$VERSION" --notes-file -

# Shows the GitHub release for the current version (Windows)
[windows]
show-release:
    $version = (Select-String -Path 'crates/bamboo/Cargo.toml' -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value; gh release view "v$version"

# Shows the GitHub release for the current version (Unix)
[unix]
show-release:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION=$(grep '^version = ' crates/bamboo/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    gh release view "v$VERSION"

# Deletes a GitHub release (by tag, e.g. v0.1.11) (Windows)
[windows]
strip-release tag:
    gh release delete {{tag}} --yes
    Write-Host ""
    Write-Host "To delete the git tag as well, run:" -ForegroundColor Green
    Write-Host "  just strip-tag {{tag}}" -ForegroundColor Green

# Deletes a GitHub release (by tag, e.g. v0.1.11) (Unix)
[unix]
strip-release tag:
    gh release delete {{tag}} --yes
    @echo ""
    @echo "To delete the git tag as well, run:"
    @echo "  just strip-tag {{tag}}"

# Bumps the patch version, updates changelog, and creates a git tag (Windows)
[windows]
bump-patch-version:
    $currentVersion = (Select-String -Path 'crates/bamboo/Cargo.toml' -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value; $parts = $currentVersion.Split('.'); $newPatch = [int]$parts[2] + 1; $newVersion = "$($parts[0]).$($parts[1]).$newPatch"; Write-Host "Bumping version from $currentVersion to $newVersion"; (Get-Content 'crates/bamboo/Cargo.toml') -replace "^version = `"$currentVersion`"", "version = `"$newVersion`"" | Set-Content 'crates/bamboo/Cargo.toml'; (Get-Content 'crates/bamboo-cli/Cargo.toml') -replace "^version = `"$currentVersion`"", "version = `"$newVersion`"" | Set-Content 'crates/bamboo-cli/Cargo.toml'; git add crates/bamboo/Cargo.toml crates/bamboo-cli/Cargo.toml; git commit -m "chore: bump version to v$newVersion"; git cliff --tag "v$newVersion" -o CHANGELOG.md; git add CHANGELOG.md; git commit -m "chore: update changelog for v$newVersion"; git tag "v$newVersion"; Write-Host ""; Write-Host "Version bumped and tagged! To push, run:" -ForegroundColor Green; Write-Host "  just push-version" -ForegroundColor Green

# Bumps the patch version, updates changelog, and creates a git tag (Unix)
[unix]
bump-patch-version:
    #!/usr/bin/env bash
    set -euo pipefail
    CURRENT_VERSION=$(grep '^version = ' crates/bamboo/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    IFS='.' read -ra PARTS <<< "$CURRENT_VERSION"
    NEW_PATCH=$((PARTS[2] + 1))
    NEW_VERSION="${PARTS[0]}.${PARTS[1]}.$NEW_PATCH"
    echo "Bumping version from $CURRENT_VERSION to $NEW_VERSION"
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" crates/bamboo/Cargo.toml
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" crates/bamboo-cli/Cargo.toml
    git add crates/bamboo/Cargo.toml crates/bamboo-cli/Cargo.toml
    git commit -m "chore: bump version to v$NEW_VERSION"
    git cliff --tag "v$NEW_VERSION" -o CHANGELOG.md
    git add CHANGELOG.md
    git commit -m "chore: update changelog for v$NEW_VERSION"
    git tag "v$NEW_VERSION"
    echo ""
    echo "Version bumped and tagged! To push, run:"
    echo "  just push-version"

# Bumps the minor version, updates changelog, and creates a git tag (Windows)
[windows]
bump-minor-version:
    $currentVersion = (Select-String -Path 'crates/bamboo/Cargo.toml' -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value; $parts = $currentVersion.Split('.'); $newMinor = [int]$parts[1] + 1; $newVersion = "$($parts[0]).$newMinor.0"; Write-Host "Bumping version from $currentVersion to $newVersion"; (Get-Content 'crates/bamboo/Cargo.toml') -replace "^version = `"$currentVersion`"", "version = `"$newVersion`"" | Set-Content 'crates/bamboo/Cargo.toml'; (Get-Content 'crates/bamboo-cli/Cargo.toml') -replace "^version = `"$currentVersion`"", "version = `"$newVersion`"" | Set-Content 'crates/bamboo-cli/Cargo.toml'; git add crates/bamboo/Cargo.toml crates/bamboo-cli/Cargo.toml; git commit -m "chore: bump version to v$newVersion"; git cliff --tag "v$newVersion" -o CHANGELOG.md; git add CHANGELOG.md; git commit -m "chore: update changelog for v$newVersion"; git tag "v$newVersion"; Write-Host ""; Write-Host "Version bumped and tagged! To push, run:" -ForegroundColor Green; Write-Host "  just push-version" -ForegroundColor Green

# Bumps the minor version, updates changelog, and creates a git tag (Unix)
[unix]
bump-minor-version:
    #!/usr/bin/env bash
    set -euo pipefail
    CURRENT_VERSION=$(grep '^version = ' crates/bamboo/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    IFS='.' read -ra PARTS <<< "$CURRENT_VERSION"
    NEW_MINOR=$((PARTS[1] + 1))
    NEW_VERSION="${PARTS[0]}.$NEW_MINOR.0"
    echo "Bumping version from $CURRENT_VERSION to $NEW_VERSION"
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" crates/bamboo/Cargo.toml
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" crates/bamboo-cli/Cargo.toml
    git add crates/bamboo/Cargo.toml crates/bamboo-cli/Cargo.toml
    git commit -m "chore: bump version to v$NEW_VERSION"
    git cliff --tag "v$NEW_VERSION" -o CHANGELOG.md
    git add CHANGELOG.md
    git commit -m "chore: update changelog for v$NEW_VERSION"
    git tag "v$NEW_VERSION"
    echo ""
    echo "Version bumped and tagged! To push, run:"
    echo "  just push-version"

# Bumps the major version, updates changelog, and creates a git tag (Windows)
[windows]
bump-major-version:
    $currentVersion = (Select-String -Path 'crates/bamboo/Cargo.toml' -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value; $parts = $currentVersion.Split('.'); $newMajor = [int]$parts[0] + 1; $newVersion = "$newMajor.0.0"; Write-Host "Bumping version from $currentVersion to $newVersion"; (Get-Content 'crates/bamboo/Cargo.toml') -replace "^version = `"$currentVersion`"", "version = `"$newVersion`"" | Set-Content 'crates/bamboo/Cargo.toml'; (Get-Content 'crates/bamboo-cli/Cargo.toml') -replace "^version = `"$currentVersion`"", "version = `"$newVersion`"" | Set-Content 'crates/bamboo-cli/Cargo.toml'; git add crates/bamboo/Cargo.toml crates/bamboo-cli/Cargo.toml; git commit -m "chore: bump version to v$newVersion"; git cliff --tag "v$newVersion" -o CHANGELOG.md; git add CHANGELOG.md; git commit -m "chore: update changelog for v$newVersion"; git tag "v$newVersion"; Write-Host ""; Write-Host "Version bumped and tagged! To push, run:" -ForegroundColor Green; Write-Host "  just push-version" -ForegroundColor Green

# Bumps the major version, updates changelog, and creates a git tag (Unix)
[unix]
bump-major-version:
    #!/usr/bin/env bash
    set -euo pipefail
    CURRENT_VERSION=$(grep '^version = ' crates/bamboo/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    IFS='.' read -ra PARTS <<< "$CURRENT_VERSION"
    NEW_MAJOR=$((PARTS[0] + 1))
    NEW_VERSION="$NEW_MAJOR.0.0"
    echo "Bumping version from $CURRENT_VERSION to $NEW_VERSION"
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" crates/bamboo/Cargo.toml
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$NEW_VERSION\"/" crates/bamboo-cli/Cargo.toml
    git add crates/bamboo/Cargo.toml crates/bamboo-cli/Cargo.toml
    git commit -m "chore: bump version to v$NEW_VERSION"
    git cliff --tag "v$NEW_VERSION" -o CHANGELOG.md
    git add CHANGELOG.md
    git commit -m "chore: update changelog for v$NEW_VERSION"
    git tag "v$NEW_VERSION"
    echo ""
    echo "Version bumped and tagged! To push, run:"
    echo "  just push-version"
