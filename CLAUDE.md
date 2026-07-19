# Open Sober — Agent Instructions

## Branch strategy

- **`stable`** — Release channel. Only merged from `dev` after tests pass.
- **`dev`** — All development happens here. Single integration branch.
- **NO worktrees or feature branches.** Do not create worktrees or separate branches.
  Work on `dev` directly. Commit early, commit often.

## How to work

1. Always work on `dev` branch.
2. Never create git worktrees or feature branches.
3. If you need to research something, do it inline or in a temp dir outside the repo.
4. Write tests for everything.
5. Make sure the full workspace compiles: `cargo check --workspace`
6. Run all tests: `cargo test --workspace`
7. After verified, commit to `dev` and push.

## Merge process

Only merge `dev` → `stable` when the user explicitly asks to ship.

## Key people

- Repo: github.com/glm-5-turbo/open-sober
- License: MIT