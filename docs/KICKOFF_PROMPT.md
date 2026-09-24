# Kickoff prompt

Copy the prompt below into Claude Code (VS Code extension, or `claude` in the terminal
from the repo root) after placing this package in an empty repository.

---

You're starting a new project. Read `CLAUDE.md` and every file in `docs/` before doing
anything else.

Then:
1. Summarize back to me, in under 15 bullet points, what we're building, the locked stack,
   and the golden rules, so I can confirm you've understood.
2. List any contradictions, gaps, or risky assumptions you see in the docs.
3. Propose a step-by-step plan for **Milestone M0 — Scaffold** from `docs/ROADMAP.md`,
   including the exact crates and npm packages you intend to add.

Stop and wait for my approval before writing code. After I approve, implement M0 in small
commits, tick off the checklist in `docs/ROADMAP.md`, update "Current status" in
`CLAUDE.md`, and tell me how to run the app and the tests.

---

## Prompts for later milestones

> Continue with the next unchecked milestone in `docs/ROADMAP.md`. Propose a plan first,
> wait for approval, then implement with tests. Update the checklist and CLAUDE.md status.

> Run the full test suite and clippy. Fix anything failing before we continue.

> I want to change [X]. Write an ADR in `docs/DECISIONS.md` covering the options and
> trade-offs, and recommend one. Don't change code yet.
