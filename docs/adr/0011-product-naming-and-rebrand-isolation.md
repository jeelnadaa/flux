# ADR-0011: Product Name "Flux" and Configuration Isolation

- **Status:** Accepted
- **Deciders:** Flux Engineering
- **Date:** 2026-11-03
- **Traceability:** PLAN.md §1 (D11), §4

## Context
"Flux" has been chosen as the working product name. However, pre-existing trademarks exist in the digital audio and AI spaces (most notably "FLUX::", a long-standing French pro-audio brand specialized in spatial audio tools, and "FLUX", an open-weights image model family). Trademark searches must be executed before public commercial deployment. If a trademark conflict forces a name change, a hardcoded product name across hundreds of files would make rebranding costly and error-prone.

## Decision
1. Retain "Flux" as the product working title.
2. Queue trademark searches and conflict audits in `PROGRESS.md` under the human gate queue `[H]`.
3. Isolate the user-facing product name, brand strings, and domains in a centralized configuration constant (`PRODUCT_NAME = "Flux"`) in `@flux/ui` / `@flux/app-core`.
4. Avoid scattering literal `"Flux"` strings across UI component copy, preset definitions, and internal routing logic, so that renaming the product requires changing a single configuration file.

## Consequences
- **Positive:** Rebranding cost drops from days of error-prone global search-and-replace to a single-line configuration change; eliminates trademark conflict blockage during early development.
- **Negative:** Requires discipline to always reference brand constants rather than hardcoding "Flux" in prose and component copy.
