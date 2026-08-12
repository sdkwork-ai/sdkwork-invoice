# Repository Guidelines

This repository is an SDKWork application root. Global standards live in
`../sdkwork-specs/`; local contracts narrow those standards but never replace them.

## SDKWORK Soul

Read `../sdkwork-specs/SOUL.md` before work in this root. Apply specs before memory, dictionary
before context, exact sources before inference, and evidence before completion.

## SDKWORK Standards

The standards entrypoint is `../sdkwork-specs/README.md`; agent behavior follows
`../sdkwork-specs/AGENTS_SPEC.md`. Use dynamic progressive loading and read only task-specific and
language-specific specs before implementation files.

## Application Identity

- Domain and capability: `commerce.invoices`
- Assembly: `crates/sdkwork-api-invoice-assembly`
- Standalone process: `crates/sdkwork-api-invoice-standalone-gateway`
- App route owner: `sdkwork-routes-invoice-app-api`
- App authority/family: `sdkwork-invoice-app-api` -> `sdkwork-invoice-app-sdk`
- Backend API: not declared; add it only with approved real operator requirements and operations.
- Permissions: `commerce.invoices.read`, `commerce.invoices.manage`

Application declarations live under `apps/` when present. Root Cargo, assembly, route, API, SDK,
and component manifests are the capability composition authority.

## Local Dictionary Structure

- `apis/`: owner-only API contracts and review inputs.
- `crates/`: Rust services, repositories, routes, hosts, assembly, and standalone process.
- `database/`: database lifecycle contracts; schema changes require human approval.
- `sdks/`: SDK family manifests, composed facades, and generated transports.
- `specs/`: application-wide machine contracts.
- `.sdkwork/`: source-controlled local AI metadata, never runtime state or secrets.
- `docs/`: human Canon and guides.
- `tools/`: thin deterministic materializers.

## Spec Resolution Order

1. Read this file and resolve the repository root.
2. Read the touched module's `specs/component.spec.json` and relevant root `specs/`.
3. Resolve the task row in `../sdkwork-specs/README.md`.
4. Language-specific specs are on-demand; load only the touched language with applicable globals.
5. Inspect implementation, edit narrowly, run narrow checks, then broaden verification.

## Required Specs By Task Type

- Rust/Cargo: `CODE_STYLE_SPEC.md`, `NAMING_SPEC.md`, `RUST_CODE_SPEC.md`, `TEST_SPEC.md`.
- API/SDK: `API_SPEC.md`, `SDK_SPEC.md`, `SDK_WORKSPACE_GENERATION_SPEC.md`,
  `APP_SDK_INTEGRATION_SPEC.md`, `COMPONENT_SPEC.md`, `TEST_SPEC.md`.
- List/search: add `PAGINATION_SPEC.md`.
- Security/IAM/privacy: `IAM_SPEC.md`, `SECURITY_SPEC.md`, `PRIVACY_SPEC.md`.
- Node/build scripts: `TYPESCRIPT_CODE_SPEC.md`, `PNPM_SCRIPT_SPEC.md`, `TEST_SPEC.md`.
- Workflow/release: `GITHUB_WORKFLOW_SPEC.md`, `DEPLOYMENT_SPEC.md`, `RELEASE_SPEC.md`.
- Documentation/agent entrypoints: `DOCUMENTATION_SPEC.md`, `AGENTS_SPEC.md`,
  `SDKWORK_WORKSPACE_SPEC.md`.

## Int64 Wire Contract (API_SPEC §13.6)

- OpenAPI `int64` fields and parameters `MUST` be `type: string`, `format: int64`,
  a decimal `pattern` such as `^-?[0-9]+$`, and `x-sdkwork-int64-string: true`.
  `type: integer, format: int64` is a contract violation: generated TypeScript
  SDKs then emit `number`, and browsers silently round ids past
  `Number.MAX_SAFE_INTEGER` (2^53), replaying wrong ids into lookups.
- Rust response DTOs `MUST` serialize `i64` wire fields with
  `#[serde(with = "sdkwork_utils_rust::serde_int64")]` (or `::option`); request
  boundaries parse inbound strings with the same helper.
- Generated TypeScript SDKs keep `int64` as `string`; frontend code `MUST NOT`
  convert ids/snowflake ids/sequence ids to `number` for storage, comparison,
  or submission.
- Verification: `node <sdkwork-specs>/tools/check-api-operation-patterns.mjs --workspace .`

## Code Style Rules

- Keep Rust `lib.rs` limited to declarations, re-exports, and lightweight wiring.
- Use canonical lower-kebab Cargo/package keys and lower-snake Rust module names.
- Reuse `sdkwork-utils` and framework helpers before adding local equivalents.
- Never hand-edit generated SDK output.

## Agent Execution Rules

Do not create an Invoice backend route crate or SDK until approved requirements declare at least one
real operation. The route manifest, owner-only OpenAPI, SDK manifest, Cargo workspace, and component specs
must agree. Generated output under `sdks/**/generated/server-openapi` is generator-owned and must not
be hand-edited. HTTP success and error shapes are defined only by `API_SPEC.md`; list operations use
store-level pagination defined by `PAGINATION_SPEC.md`. Capability assemblies export business routes
only; the host owns `/healthz`, `/livez`, `/readyz`, and `/metrics`.

## Documentation Canon

- [docs/README.md](docs/README.md)
- [docs/product/prd/PRD.md](docs/product/prd/PRD.md)
- [docs/architecture/tech/TECH_ARCHITECTURE.md](docs/architecture/tech/TECH_ARCHITECTURE.md)

## Build, Test, and Verification

```powershell
cargo metadata --no-deps --format-version 1
cargo fmt -- --check
cargo test --workspace
cargo clippy --workspace --tests -- -D warnings
pnpm check
node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node ../sdkwork-specs/tools/check-pagination.mjs --workspace .
node ../sdkwork-specs/tools/check-permission-composition.mjs --root .
node ../sdkwork-specs/tools/check-app-sdk-consumer-imports.mjs --workspace .
```

## Human Review Rules

Human review is required for breaking public API/SDK or standards changes, security exceptions,
database schema or migration changes, destructive filesystem work, release policy changes, and
generated SDK ownership changes.
