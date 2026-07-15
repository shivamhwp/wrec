# Security

Wrec records the screen, so treat any vulnerability that captures content
without consent, escapes the daemon socket boundary, or tampers with recording
output as high severity.

## Reporting

Open a [GitHub issue](https://github.com/shivamhwp/wrec/issues/new) and
include:

- what the issue is and where (app, CLI, daemon, capture engine, installer)
- steps to reproduce
- the impact you believe it has

You will get a response within a week.

## Scope notes

- The daemon listens on a unix socket at `~/.wrec/wrec.sock` and trusts local
  processes running as the same user. Anything that lets a *different* user or
  a sandboxed process drive recordings is a bug.
- Release artifacts are not notarized and only ad-hoc signed, so macOS
  Gatekeeper warns on the first launch of the app DMG. Download only from the
  official GitHub Releases page; each release asset publishes a SHA-256 digest
  (`gh release view <tag> --json assets --jq '.assets[].digest'`) you can verify
  a manual download against. Then clear the quarantine as described in the
  README.
