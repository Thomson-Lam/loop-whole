# `read` token reduction

## Purpose

The `read` tool returns the requested UTF-8 file view while remembering the last result delivered for the same request during the active gateway session.

A read baseline is identified by:

```text
canonical path + offset + limit
```

Different offsets or limits are different baselines. Overlapping-range reasoning is not implemented.

## Original and intercepted payloads

For every invocation:

- `original` is the complete bounded result that the normal read would return now;
- `intercepted` is the exact result delivered to the agent.

The normal read result is limited to 2,000 lines or 50KB and includes continuation metadata when applicable.

## Delivery decisions

### First observation

```text
deliveryMode: full
decisionReason: no_read_baseline
```

`original` and `intercepted` are identical, so output savings should be zero.

### Repeated unchanged view

If the current view hash equals the stored view hash:

```text
deliveryMode: unchanged
decisionReason: requested_view_unchanged
```

The agent receives a short unchanged marker instead of the full requested view.

If the complete file changed but the requested view did not, the reason is:

```text
requested_view_unchanged_file_changed
```

This does not claim that the whole file is unchanged.

### Repeated changed view

The gateway computes a unified line diff from the previous delivered view to the current view.

If the estimated diff is smaller than the current full view:

```text
deliveryMode: diff
decisionReason: requested_view_changed
```

For partial or truncated views, the reason is `partial_requested_view_changed`.

If the diff is not smaller, the current view is returned instead:

```text
deliveryMode: full
decisionReason: diff_not_smaller_than_current_view
```

After every successful read, the current view becomes the next baseline.

## Token calculation

```text
originalOutputTokens    = ceil(original characters / 4)
interceptedOutputTokens = ceil(intercepted characters / 4)
savedTokens             = originalOutputTokens - interceptedOutputTokens
```

The session-wide percentage also includes unchanged tool-input tokens in its denominator.

## Diagnosis

Expected smoke sequence:

```text
first read:    full       savings near zero
second read:   unchanged  positive savings
edit + reread: diff       positive savings when the diff is smaller
```

If a repeated read remains `full`, check that path, offset, and limit are exactly identical and that both calls used the same live gateway session.

See `@docs/tests/manual.md` and `@tests/opencode/instructions/01-read-unchanged.md`.
