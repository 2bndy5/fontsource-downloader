# Contribution guidelines

First, thank you for considering a contribution to this project!

This document should help detail the project's expectations about contributions.

## Development tools

This project uses the following tools for development:

- [cargo-llvm-cov] for measuring code coverage.
- [cargo-nextest] as a testing harness.
- [nur] for running common tasks in the development workflow.

### Optional tools

- [committed] for verifying commit messages conform to [conventional commit]
  standards.
- [git-cliff] for generating a [Changelog](CHANGELOG.md) and release notes.
- [pre-commit] for sanitizing project files.

[cargo-llvm-cov]: https://crates.io/crates/cargo-llvm-cov
[cargo-nextest]: https://crates.io/crates/cargo-nextest
[nur]: https://crates.io/crates/nur
[committed]: https://crates.io/crates/committed
[conventional commit]: https://www.conventionalcommits.org
[git-cliff]: https://crates.io/crates/git-cliff
[pre-commit]: https://pre-commit.com
[prek]: https://github.com/j178/prek

## Submitting patches

Please, please, please open an issue to discuss possible solutions before
submitting a Pull Request. If it is a small patch (ie 1 or 2 lines), then a
preemptive issue may not be warranted. Although, it still helps to first discuss
the reason of the small patch in some manor.

Pull Request titles should conform to [conventional commits] standard. Upon
merging the Pull Request, all commits on the feature branch are squashed into a
single commit pushed to the default (main) branch. This is done so [git-cliff]
can adequately generate a list of changes when processing a release.

## Code style

[uv]: https://docs.astral.sh/uv
[pipx]: https://pipx.pypa.io/stable

This project's CI leverages [pre-commit] to ensure

- [x] line ending all use LF (not CRLF)
- [x] lines have no trailing whitespace
- [x] files end with a blank line
- [x] valid syntax is used in all yaml and toml files
- [x] no large files (greater than 500 kB) are added
- [x] no unknown or misspelled words are present

Normally, [pre-commit] is typically run from a Python virtual environment. This
project has no other practical need for a Python virtual environment. Instead,
[pre-commit] can be run as with a one-line command using [uv] or [pipx].

```shell
pipx run pre-commit run --all-files
```

```shell
uvx pre-commit run --all-files
```

The [nur] `pre-commit` task actually uses the [prek] (drop-in
compatible) alternative to pre-commit.

```shell
nur pre-commit
```

Optional arguments are documented and shown in `nur pre-commit -h`.

### Static analysis

Code format and linting is done by using `cargo clippy` and `cargo fmt`. Both
commands are performed using a [nur] task as well:

```shell
nur lint
```

## Testing

To simplify the lengthy commands to run [cargo-llvm-cov] and [cargo-nextest]
tools in tandem, we use [nur] to parametrize the various options applicable to
this project into tasks.

### Running tests

Unit tests are performed using [cargo-nextest] while coverage is measured by
[cargo-llvm-cov].

#### Run the tests

```shell
nur test
```

Optional arguments are documented and shown in `nur test -h`.

The `default` test profile skips tests that are known to run longer than 10
seconds. To use the CI test profile (which includes slow tests), simply pass
`--profile ci` (or `-p ci`) to the `nur test` command:

```shell
nur test -p ci
```

Python tests can also be run with

```shell
nur test py
```

Optionally pas `-c` to reinstall the python binding.

#### Generate coverage data (for codecov)

```shell
nur test lcov
```

#### Generate local HTML coverage report

```shell
nut test html
```

Optional arguments are documented and shown in `nur test html -h`.

## API documentation

Documentation is hosted at docs.rs automatically upon release. To verify any
documentation changes locally, we can use [nur] for that too:

```shell
nur docs
```

Optional arguments are documented and shown in `nur docs -h`.
