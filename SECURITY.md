# Security Policy

## Supported versions

Security fixes are provided for the latest onley release. Upgrade to the latest
release before you report a problem. Older releases do not receive security
fixes.

| Version | Supported |
| --- | --- |
| Latest release | Yes |
| Older releases | No |

## Reporting a vulnerability

Do not open a public issue for a security problem. Email hello@getfda.dev with
the subject prefix `[SECURITY]`. Reports may be written in English or Chinese.

Include:

- A description of the problem and its impact.
- The onley version or release tag, operating system, and architecture.
- The command or UI action that reproduces the problem.
- A minimal reproduction or proof of concept, if you have one.
- Any suggested fix or mitigation.

Do not send tokens, passwords, or private file contents. A minimal reproduction
is enough.

We will acknowledge the report as soon as possible, investigate it, and keep you
informed of the progress. Please give us a reasonable time to fix the problem
before you disclose it publicly.

## Scope

This policy covers the onley CLI, the desktop client, and the release artifacts
built from this repository.

Security-relevant problems include:

- Unauthorized deletion, overwrite, or disclosure of files.
- Path traversal, symlink following, or writes outside the selected directory or
  replica store.
- A flaw in replica mode that lets a remote client act beyond the documented
  protocol.
- Remote code execution or command injection.
- Compromise of release artifacts or signing credentials.

## Deployment notes

onley runs locally. It has no telemetry and does not send data to a server unless
you run `serve`, `replica`, or another network command.

Replica mode exposes an HTTP API without authentication or TLS. This is a known
limitation, not a vulnerability by itself. Anyone who can reach the port can
query the index and upload files. Run `serve` only on a trusted network, or put
it behind a reverse proxy, VPN, or firewall. Do not expose the port to the
public internet.

## Out of scope

- Problems that require an attacker to already have full control of the machine
  or the user account.
- Reports that only demonstrate the documented lack of authentication or TLS in
  replica mode, without a further impact.
- Reports about third-party dependencies that are already fixed in the latest
  release; report those upstream first.
- Social engineering, spam, or physical attacks.
- Issues in modified binaries or unsupported builds.
