# NautPie (Rust)

A single-binary Rust CLI for talking to the DeployNaut API and Bitbucket Pipelines.
Behaviour-compatible port of the original `nautpie.phar` (PHP/Symfony).

## Conversion from PHP

The original `nautpie.phar` was a Symfony Console application written in PHP,
dispatching two commands (`deploy:naut`, `ci:bitbucket`) and using Guzzle/Ring
through a `CurlFetch` helper. This repository is a ground-up Rust port — the
PHP source has been removed entirely and replaced with a single binary that
preserves the public CLI surface, the JSON-line stdout contract, and the env-var
interface.

### Stack

| Concern | PHP original | Rust port |
| --- | --- | --- |
| CLI dispatch | Symfony Console | [`clap`](https://crates.io/crates/clap) 4.6 (derive) |
| HTTP client | Guzzle/Ring + curl | [`reqwest`](https://crates.io/crates/reqwest) 0.13 (`blocking`, rustls) |
| Progress UI | Repeated stderr `"Waiting for 5 seconds..."` lines | [`ratatui`](https://crates.io/crates/ratatui) 0.29 + [`crossterm`](https://crates.io/crates/crossterm) 0.28, with non-TTY stderr fallback |
| `.env` loading | `vlucas/phpdotenv` `safeLoad` | [`dotenvy`](https://crates.io/crates/dotenvy) |
| Time parsing | PHP `strtotime` | [`chrono`](https://crates.io/crates/chrono) 0.4 |
| Errors | ad-hoc exceptions | [`thiserror`](https://crates.io/crates/thiserror) enum (`Error::MissingEnv`, `MissingOption`, `MissingAction`, `MissingEndpoint`, `Timeout`, `HttpStatus`, `Json`, `Generic`) |
| Stderr colouring | `SymfonyStyle` | [`owo-colors`](https://crates.io/crates/owo-colors) |

`reqwest` is intentionally synchronous (matches PHP). `ratatui` was chosen over
`tui-rs` because `tui-rs` was abandoned in 2023.

### Behaviour parity notes

The port is byte-for-byte compatible on the wire — same JSON envelope, same env
vars, same option names. A few deliberate changes were made and are documented
here so consumers aren't surprised:

- **`summary` typo corrected** — the PHP `DeploymentDetails::summery()` builder
  method (a misspelling) serialised the field as `summary` at the HTTP layer. The
  Rust builder is named `summary` end-to-end.
- **`do_deploy_package` returns a single combined response** — PHP printed three
  separate JSON lines (token, deployment, response). The Rust port returns one
  `ApiResponse` matching the create-deployment call.
- **Action names are case-insensitive at the CLI boundary** —
  `sampleSuccess`, `SampleSuccess`, `SAMPLE_SUCCESS`, and `sample_success` all
  resolve to the same action, matching PHP's `ucfirst` + case-insensitive
  method lookup.
- **TTY detection is automatic** — the spinner uses ratatui when stderr is a TTY
  and falls back to a 5-second stderr line otherwise (matches PHP behaviour in
  CI).
- **`fetch_deployments` date parser** — supports `-1 year/month/day`,
  `yesterday`, `last year/month`, RFC3339, Unix epoch, and "now" fallback. The
  full PHP `strtotime` grammar is not implemented; exotic expressions will fall
  through to the current time.

### Test parity

The PHPUnit suite was reimplemented as Cargo integration tests:

- `tests/cli_smoke.rs` — `tests/php/...Test.php` smoke tests (5 cases).
- `tests/http_test.rs` — replaces Guzzle `MockHandler` with
  [`wiremock`](https://crates.io/crates/wiremock) (7 cases: GET, POST-JSON,
  POST-form, 4xx, Basic auth, missing endpoint).
- `tests/deploy_naut_test.rs` — one case per `deploy:naut` action
  (`fetch`, `createDeployment`, `gitFetch`, `getDeployments`, `lastDeployment`)
  matching the PHPUnit cases (13 tests).
- `tests/bitbucket_test.rs` — `createAccessToken`, `createTag` (with the PHP
  substring-truncation behaviour), and `deployPackage` (7 tests).

JSON fixtures in `tests/fixtures/*.json` are unchanged from the PHP repo.

### Project layout

```text
src/
  main.rs                      entrypoint + dispatcher
  lib.rs                       public crate surface
  cli.rs                       clap subcommands + action normalisation
  error.rs                     Error + ApiResponse
  env.rs                       load_dotenv, check_envs
  boolean.rs                   check_boolean
  io/                          Io trait + StderrIo
  http/                        HttpClient trait + ReqwestHttpClient
  deployment_details.rs        fluent DeploymentDetails builder
  options.rs                   options bag
  tui/spinner.rs               ratatui progress widget + non-TTY fallback
  commands/
    sample.rs                  sampleSuccess / sampleFail
    deploy_naut.rs             fetch / createDeployment / gitFetch / getDeployments / lastDeployment
    bitbucket.rs               createAccessToken / createTag / deployPackage
tests/
  fixtures/                    JSON fixtures (copied from PHP repo)
  cli_smoke.rs, http_test.rs, deploy_naut_test.rs, bitbucket_test.rs
.github/workflows/ci.yml       runs fmt + clippy + release build + test on push
```

### Continuous integration

`.github/workflows/ci.yml` runs on every push and pull request:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo build --release --locked`
4. `cargo test --all-targets --locked`

The release binary is uploaded as a workflow artifact on every green run.

## Install

```sh
cargo build --release
# binary at target/release/nautpie
```

## Usage

```sh
nautpie --help

# DeployNaut actions (require NAUT_ENDPOINT, DASH_USER, DASH_TOKEN)
nautpie deploy:naut createDeployment \
    --stack=example --environment=uat \
    --ref=40char-sha --ref_type=sha \
    --bypass_and_start=true --should_wait=true

nautpie deploy:naut gitFetch --stack=example
nautpie deploy:naut getDeployments --stack=example --environment=uat
nautpie deploy:naut lastDeployment --stack=example --environment=uat
nautpie deploy:naut fetch --url=meta

# Bitbucket actions (require BB_ENDPOINT, BB_AUTH_STRING, etc.)
nautpie ci:bitbucket createAccessToken
nautpie ci:bitbucket createTag --commit=40char-sha --tag=v1.2.3
nautpie ci:bitbucket deployPackage --stack=example --environment=uat --commit=40char-sha

# Sample (smoke-test) actions — no network calls
nautpie deploy:naut sampleSuccess
nautpie deploy:naut sampleFail
```

## Output

Every command prints exactly one JSON line on stdout:

```json
{"status":200,"reason":"OK","body":...}
```

Errors are reported as:

```json
{"status":<code>,"reason":"Bad Request","body":"<message>"}
```

Status `0` means success; non-zero indicates a failed action.

## Environment variables

| Variable | Required by |
|---|---|
| `NAUT_ENDPOINT` | `deploy:naut` |
| `DASH_USER`, `DASH_TOKEN` | `deploy:naut` |
| `BB_ENDPOINT` | `ci:bitbucket` |
| `BB_AUTH_STRING` | `ci:bitbucket` |
| `BB_CONSUMER_KEY`, `BB_CONSUMER_SECRET` | `ci:bitbucket createAccessToken` |
| `BITBUCKET_REPO_OWNER`, `BITBUCKET_REPO_SLUG` | `ci:bitbucket createTag`, `deployPackage` |
| `BITBUCKET_BRANCH` | `ci:bitbucket deployPackage` |

A `.env` file in the current working directory is loaded automatically.

## Building the learning book (PDF)

The repository ships a guided tour of the codebase organised by Rust concept rather than by file. `index.md` is the table of contents; every chapter links to beginner-friendly markdown files alongside the source (`src/*.md`, `tests/*.md`, `examples/README.md`).

You can read those markdown files directly on GitHub, or render them into a single book-style PDF (~131 pages, ~4 MB) with:

```sh
# One-time dependency (Python 3.10+)
pip install markdown Pygments

# Build the PDF (uses headless Chrome for rendering)
python3 scripts/build_pdf.py
```

The script:

1. Reads `index.md` and walks every `.md` file it references.
2. Concatenates them into one styled HTML document with chapter tags, table of contents, and Pygments syntax highlighting.
3. Renders the final PDF via `google-chrome --headless --print-to-pdf`.

Output:

- `nautpie-learning-book.pdf` — the final book (book-style A4 with running chapter headers and `page N of M` footers).
- `build/nautpie-learning-book.html` — the intermediate HTML if you want to inspect or re-style the output.

### Requirements

| Tool | Used for |
|---|---|
| Python 3.10+ | Running `scripts/build_pdf.py`. |
| `markdown` (PyPI) | Converting `.md` files to HTML. |
| `Pygments` (PyPI) | Syntax highlighting Rust, TOML, JSON, etc. in code blocks. |
| `google-chrome` (or any recent Chromium) | Rendering HTML to PDF via `--print-to-pdf`. |

Any modern Chromium with `--headless=new --print-to-pdf` support will work as a substitute for `google-chrome`.

### Editing the book

The book is just rendered markdown. To change what appears:

- **Reorder or add chapters** — edit `index.md`. The PDF reflects whatever `index.md` references.
- **Change the look** — edit the `CSS` block in `scripts/build_pdf.py`. It's a single string at the top of the file.
- **Add or change a chapter's content** — edit the corresponding `src/*.md` (or `tests/*.md`, etc.) and re-run the script.

The script does no caching, so a fresh run always reflects the current state of the markdown files.

## License

MIT. See `LICENSE.txt`.
