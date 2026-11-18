# Repository Guidelines

## Project Structure & Module Organization
FlowGram is a Rush-managed monorepo. Production-ready libraries live under `packages/*` (canvas engine, node engine, runtime, plugins). Demo UIs and docs sit inside `apps/*` (for example `apps/demo-free-layout`, `apps/docs`). Shared tooling, config, and scripts live under `common/` and `config/`. End-to-end Playwright suites are isolated in `e2e/<scenario>` so they can be installed and run independently. New code should land in the closest existing package; create additional Rush projects only when a module needs its own version and publish cycle.

## Build Model
Each library package compiles `src/` to `dist/` via tsup (`build:fast`; `build` adds `--dts-resolve`), and `package.json#exports` points at `dist`, not `src`. Workspace consumers therefore read built output: after editing a package, rebuild it (`cd packages/<category>/<pkg> && rushx build:watch`) or run the demo through its `rush dev:*` command, which pairs `rush build:watch --to-except <app>` with the app's dev server. Demos build with Rsbuild (the `demo-nextjs*` apps use Next.js, `demo-vite` uses Vite), `apps/desktop` uses Vite, and `apps/docs` is its own site. Single package: `cd packages/<category>/<pkg> && rushx build | test | ts-check | lint`.

## Build, Test, and Development Commands
Use pnpm 10.6.5 (Rush enforces versions); `nodeSupportedVersionRange` is `>=18.20.3 <19.0.0 || >=20.14.0 <23.0.0`, and CI runs Node 18. Install dependencies via `rush install`. Run `rush build` to compile every registered project. Use `rush dev:docs` or `rush dev:demo-free-layout` for hot-reload docs and demos. `rush lint`, `rush lint:fix`, and `rush ts-check` keep lint/TS diagnostics consistent. `rush test` aggregates unit tests; `rush e2e:test` runs Playwright suites, while `rush e2e:update-screenshot` refreshes snapshots.

## Desktop & Rust Backend
`apps/desktop` is the Tauri 2 shell plus a React 19 / Vite / Tailwind CSS 4 workbench; `apps/desktop/src-tauri/src` only hosts the window and the backend process. All product logic — projects, sessions, the agent loop, workflow runtime, Workflow Tools, MCP — lives in `apps/desktop-backend`, a standalone Rust HTTP crate that must not depend on Tauri, React, Node.js, or a browser (default `127.0.0.1:18789`). Rust workspace members are declared in the root `Cargo.toml`.

```sh
cargo run -p flowgram-desktop-backend --bin flowgram-server
cargo test --locked -p flowgram-desktop-backend
cargo clippy --locked -p flowgram-desktop-backend --all-targets -- -D warnings

cd apps/desktop && rushx dev          # browser dev server
cd apps/desktop && rushx desktop      # Tauri dev
cd apps/desktop && rushx desktop:build
```

Project data on disk: `flowgram.project.json`, `workflows/<workflowId>/workflow.json` (the definition source of truth), `assets/`, and `.flowgram/state.sqlite` (index and run association only). Never commit the app data directory or a project's `.flowgram/`.

## Coding Style & Naming Conventions
We write TypeScript with React, sharing configs from `config/eslint-config`. ESLint enforces 2-space indentation, semicolons, and import order; run `rush lint:fix` before committing. Use `PascalCase` for React components and classes, `camelCase` for variables/functions, and `SCREAMING_SNAKE_CASE` only for constants exported from config files. File names follow kebab-case (e.g., `flow-node-form.tsx`). Keep public API surfaces documented via barrel files such as `packages/canvas-engine/core/src/index.ts`.

## Testing Guidelines
Unit tests use Vitest and live beside source in `__tests__` folders or `*.test.ts` files; prefer descriptive names like `node-service.test.ts`. Ensure new logic is covered by `rush test`, and include data fixtures where possible. Playwright specs under `e2e/*/tests` cover critical workflows—coordinate UI changes with updated snapshots and run `rush e2e:test --to <package>` to scope failures.

```sh
# one unit test file or one test name (Vitest, per package)
cd packages/canvas-engine/core && rushx test src/__tests__/xxx.test.ts
cd packages/canvas-engine/core && rushx test -t "test name"

# one Playwright spec
cd e2e/free-layout && npx playwright test tests/xxx.spec.ts
```

## Commit & Pull Request Guidelines
Follow conventional commits (`type(scope): subject`) as seen in history (`fix(auto-layout): ...`). Keep subjects imperative and ≤72 characters, with optional bodies for context. PRs must describe the change, link GitHub issues, and attach before/after screenshots for UI updates. Confirm CI status, note any follow-ups, and request reviewers from the owning package tags (see `rush.json`).

- Commit each completed feature immediately after its relevant checks pass. Do not begin an unrelated feature while the completed feature is still uncommitted.
- Keep each commit focused on one functional change, including its necessary tests and documentation. Split shared files by feature when changes overlap; do not use broad staging to combine unrelated work.
- Before committing, inspect the staged diff and exclude generated artifacts, local data, and credentials. After committing, check the worktree and clearly distinguish any pre-existing or unfinished changes. Do not push unless requested.
- Git tag events notify the team chat. Do not create, push, or move Git tags or publish releases unless the user explicitly requests that action. Desktop preview updates must only replace assets in the existing published channel; never create or publish a channel automatically.

## CI and Pre-commit Gates
CI (`.github/workflows/ci.yml`) runs in this order on Node 18, through `node common/scripts/install-run-rush.js <cmd>`: `install` → `build` → `lint --verbose` → `ts-check` → `test:cov`. Pre-commit hooks run `rush lint-staged` and `rush commitlint`; `rush check-circular-dependency` and `rush dep-check` catch structural regressions.

## Design & Spec Documents
`design/*.md` are the authoritative specs — `desktop-app-technical-design.md`, `desktop-design-system.md`, `agent-workflow-tools.md`, `workflow-node-configuration.md`, `agent-context.md`, plus the information-architecture notes. Read the relevant one before changing desktop behaviour; do not invent contracts that contradict them. Repo skills at `.claude/skills/` (`create-node`, `material-component-dev`, `material-component-doc`) encode node and material conventions; use them rather than improvising the node data contract (`inputsValues` / `inputs` / `outputs`).

## Desktop Design Standard

- Follow the user-selected Claude design references: https://claude-storybook.bystanderss.chatgpt.site and https://www.figma.com/design/iAyueTdyOjxZZhNpRpbCQY/MCP-Apps-for-Claude--Community-. Keep implementation details and token mappings in `design/desktop-design-system.md`.
- Reuse shared surface, typography, radius, spacing and motion tokens. Prefer restrained action feedback, preserve canvas viewport and mounted editor identity, and never animate panel width during pointer dragging. Respect reduced motion.
- Use the official FlowGram logo from repository assets. Desktop settings are a full-page destination; workspace sections use bounded, keyboard-accessible, persisted resize handles.

- Desktop workflow ports must use explicit static port definitions and data-driven coordinates. Do not enable FlowGram `useDynamicPort` or discover ports by measuring or scanning DOM elements.
- Desktop UI copy must support English and Simplified Chinese with a persisted language setting and a system-language default. Language changes must preserve drafts and the mounted canvas. Translate presentation fields only; never translate IDs, enum values, editor kinds, schemas, or user-authored content.
- Agent system prompts, default agent/LLM prompt templates, Workflow Tool descriptions, shared node contracts, and backend diagnostics must be English. Agents should match the user's requested response language independently of the UI language.
- Use the shared desktop `ui/select` primitive for visible dropdowns. Do not add native `<select>` or `<option>` UI; keep option values stable across themes and locales. Canvas background, dots, node surfaces, and lines must use theme tokens without reinitializing the editor.

- Deployment belongs to full-page Settings as one global module with Releases, Credentials, and Runtime services. Keep project releases as persistent cards with version history and per-target deployments; global credentials explicitly grant all-project or selected-project availability. Do not add a project-scoped Services sidebar destination or a Current project group inside settings. Workflow publish actions deep-link into this same module and preserve the mounted workspace.
