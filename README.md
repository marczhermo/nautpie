# NautPie (Rust)

A single-binary Rust CLI for talking to the DeployNaut API and Bitbucket Pipelines.
Behaviour-compatible port of the original `nautpie.phar` (PHP/Symfony).

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

## License

MIT. See `LICENSE.txt`.
