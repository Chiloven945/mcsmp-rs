# Security policy

## Supported versions

Before the first release, security fixes are applied to the default development
branch. Published release support windows and backport policy will be listed
here once stable versions exist.

## Security model

`mcsmp-rs` connects to a privileged Minecraft management endpoint. Anyone who
can successfully use that endpoint may be able to inspect players, change
allowlists and bans, alter live settings, send messages, save worlds, or request
server shutdown. Treat access to the management secret and endpoint as
administrative access.

The library's security-oriented behavior includes:

- explicit authentication selection; a builder cannot connect until it has a
  bearer, subprotocol, or deliberate no-auth mode;
- header-safety validation for secrets and redacted `Secret` debug output;
- normal TLS certificate and hostname validation for `wss://` URLs;
- no public "accept any certificate" bypass;
- no automatic replay of management requests after disconnection; and
- parser and fuzz coverage for untrusted inbound JSON-RPC frames.

These properties do not eliminate the need for secure server configuration,
network isolation, access control, logging hygiene, and dependency updates.

## Deployment guidance

- Prefer `wss://` and a certificate trusted by the client environment.
- Bind the management endpoint to loopback or a tightly controlled private
  network unless a stronger network-access design is in place.
- Configure `management-server-allowed-origins` deliberately and pass the same
  value through `ClientBuilder::origin`.
- Use a separate, high-entropy management secret per server.
- Do not place secrets in source control, examples, fixtures, logs, crash
  reports, or telemetry.
- Treat timeouts and disconnects as ambiguous outcomes; query server state
  before retrying a privileged operation.
- Keep dependencies, the Rust toolchain, and Minecraft server software current.

See [server configuration](docs/server-configuration.md) and
[error handling](docs/error-handling.md) for operational guidance.

## Reporting a vulnerability

Do **not** open a public issue for a suspected credential leak, TLS bypass,
request replay flaw, memory-safety issue, denial of service, data exposure, or
privilege escalation.

Report it privately to the repository maintainers through the contact method on
the repository profile. Include:

1. affected `mcsmp-rs` versions or commit hashes;
2. a concise description of the issue and its practical impact;
3. reproduction steps or a minimal proof of concept;
4. relevant configuration details with secrets removed;
5. any mitigation or patch suggestion; and
6. a secure contact method for follow-up.

Do not include `management-server-secret`, live server addresses that should
remain private, personal data, or other sensitive credentials in reports.

## Response expectations

Maintainers will acknowledge a credible report, assess impact, work on a fix,
and coordinate disclosure where appropriate. The exact timeline depends on
severity, reproducibility, and availability of a safe remediation.
