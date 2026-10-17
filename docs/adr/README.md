# Architecture Decision Records (ADRs)

Decisions in Pinacoteca that carry a real trade-off are recorded here. The
purpose is to keep *why* a choice was made next to *what* was chosen, so the
reasoning survives across sessions and does not have to be re-derived.

## Format

Each ADR is a short Markdown file named `NNNN-short-title.md`, numbered in
order. Use the template below.

```markdown
# NNNN. <Title>

- **Status:** proposed | accepted | superseded by [NNNN](NNNN-...md)
- **Date:** YYYY-MM-DD
- **Milestone:** M2
- **Deciders:** ...

## Context

What forces are at play: requirements, constraints, measurements.

## Options considered

1. <Option A> — pros / cons.
2. <Option B> — pros / cons.

## Decision

What we chose, stated plainly.

## Consequences

What becomes easier, what becomes harder, what we now must maintain.
```

## Conventions

- **Evidence beats assumption.** Where a decision depends on performance or
  behavior, cite a measurement (benchmark output, prototype result) in the ADR.
- **Alternatives are recorded**, including rejected ones, so the same ground is
  not re-litigated.
- **Status** moves `proposed → accepted`. A decision that is later overturned is
  marked `superseded by NNNN`, never deleted.

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-decoder-strategy.md) | Image decoder strategy (pure-Rust vs native) | accepted |
| [0002](0002-gpu-zoom-render-pipeline.md) | GPU rendering and zoom (mip pyramid) | accepted |
| [0003](0003-thumbnail-cache.md) | Thumbnail cache and background workers | accepted |
| [0004](0004-database-schema-v1.md) | Database schema v1 | accepted |
| [0005](0005-concurrency-model.md) | Concurrency and threading model | accepted |
