# ADR-0007: Internationalization (i18n) from Day One

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D7), §0

## Context
Retrofitting internationalization (i18n) and right-to-left (RTL) text support onto an existing audio UI codebase is notoriously error-prone, requiring tedious text extraction and layout rework. Flux targets global creators from inception.

## Decision
1. Ban hardcoded human-facing strings in all React components from the very first UI commit.
2. Structure all labels, tooltips, dialogs, and button copy into structured message catalogs.
3. Build layout containers using CSS logical properties (`margin-inline`, `padding-inline`, `inset-inline-start`) and CSS container queries to ensure robust RTL and length-expansion support.
4. Establish English as the baseline source language with pseudo-localization testing in CI.

## Consequences
- **Positive:** Zero translation refactoring debt; seamless future addition of European, Asian, and Middle-Eastern language translations.
- **Negative:** Minor initial overhead in writing string keys rather than inline English text in templates.
