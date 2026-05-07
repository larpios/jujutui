# https://just.systems

default:
    @just -l

check-fmt:
    @echo "Checking formatting..."
    @cargo fmt -- --check

test:
    @echo "Running tests..."
    @cargo test

clippy:
    @echo "Running clippy..."
    @cargo clippy -- -D warnings

check: check-fmt test clippy
    @cargo check

fmt:
    @echo "Fixing formatting..."
    @cargo fmt

fix-lint:
    @echo "Fixing lint..."
    @cargo fix --allow-dirty

fix-clippy:
    @echo "Fixing clippy..."
    @cargo clippy --fix --allow-dirty

fix-all: fix-lint fix-clippy fmt

bump-version-to version:
    #!/usr/bin/env nu
    print "Bumping version to {{version}}..."
    open Cargo.toml | update package.version {{version}} | save -f Cargo.toml

bump-version part:
    #!/usr/bin/env nu
    print "Bumping version..."
    if not ('{{part}}' in ['major', 'minor', 'patch']) {
        print "Invalid part: {{part}}"
        exit 1
    }

    mut curr_version = open Cargo.toml | get package.version | split row '.' | each { into int }

    match '{{part}}' {
        'major' => {
            $curr_version.0 += 1
            $curr_version.1 = 0
            $curr_version.2 = 0
        }
        'minor' => {
            $curr_version.1 += 1
            $curr_version.2 = 0
        }
        'patch' => {
            $curr_version.2 += 1
        }
    }

    just bump-version-to ($curr_version | str join '.')


