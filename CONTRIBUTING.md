# Contributing to onley

Thanks for your interest in onley. This guide explains how to report a problem,
propose a change, and submit a pull request.

Issues and pull requests may be written in English or Chinese.

## Code of conduct

All participants must follow the [Code of Conduct](CODE_OF_CONDUCT.md). Report
unacceptable behavior to hello@getfda.dev. Do not open a public issue for a
conduct report.

## Ways to contribute

- Report a bug with a minimal reproduction.
- Improve the documentation or tests.
- Fix a bug or implement a focused feature.

For a large change, open an issue first so we can agree on the scope before you
write code.

## Development setup

onley requires Rust 1.88 or newer. The desktop client uses NSH. The real-window
UI test also needs Xcode and the `nsh` command.

```sh
git clone https://github.com/fdaio/onley
cd onley
make build      # build the CLI as ./onley
make test       # run all tests
make lint       # run cargo fmt --check and cargo clippy
```

The desktop client builds on macOS:

```sh
make app        # package the macOS app
```

Run the real-window UI test. The script creates a temporary index and removes
the files the test deletes:

```sh
./scripts/real-window-test.sh
```

CI runs the same format, lint and test checks on every pull request. A pull
request must pass them.

## Reporting a bug

Open an issue at https://github.com/fdaio/onley/issues and include:

- What you expected to happen and what happened instead.
- The exact command or UI action that reproduces the problem.
- The onley version or release tag, operating system, and architecture.
- Relevant log output, with personal paths and secrets removed.
- A minimal directory layout or sample files when the bug involves scanning or
  deduplication.

Do not include tokens, passwords, private file contents, or other sensitive
data in an issue.

## Submitting a pull request

1. Fork the repository, then create a branch in your fork. If you have write
   access, create a feature branch in the repository. Do not push directly to
   `master`.
2. Keep the change focused. Do not mix unrelated refactors, formatting, or
   documentation updates.
3. Add or update tests for behavior changes.
4. Run `make test` and `make lint` before you push.
5. Update `README.md` and `README_CN.md` when user-facing behavior or
   instructions change.
6. Open a pull request against `master`.

A good pull request description answers:

- What problem does this change solve?
- What is the approach?
- How did you test it? Include the commands and their result.
- What are the risks or limitations?

For a desktop UI change, add a screenshot or a short recording.

Use a short imperative commit subject. A scope prefix such as `feat:`, `fix:`,
`docs:`, `build:`, `test:`, or `style:` is welcome.

## Review

A maintainer reviews every pull request. CI must pass before merge. Review
comments may ask for a smaller change, more tests, or a different approach.
Please keep the discussion focused on the code and the problem.

## License

onley is released under the MIT License. By contributing, you agree that your
contributions are licensed under the same terms. See [LICENSE](LICENSE).

## Further reading

- [Setting guidelines for repository contributors](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/setting-guidelines-for-repository-contributors)
- [Contributor Covenant](https://www.contributor-covenant.org/)
