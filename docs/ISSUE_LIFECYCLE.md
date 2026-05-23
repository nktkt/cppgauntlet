# Issue Lifecycle

CppGauntlet keeps issue lifecycle state explicit so long-running roadmap work does not get lost or closed without context.

## Lifecycle States

New issues start with `needs-triage`. During triage, maintainers assign an area label, confirm priority, and decide whether the issue is ready for work, waiting for information, blocked, or stale.

| State | Label | Use when |
| --- | --- | --- |
| New | `needs-triage` | The issue has not been reviewed by a maintainer. |
| Needs information | `status: needs-info` | The next action belongs to the reporter or contributor. |
| Blocked | `status: blocked` | Work cannot proceed because of an external dependency, product decision, missing upstream support, or unavailable reproduction. |
| Stale | `status: stale` | The issue has been inactive long enough that maintainers need confirmation before scheduling work. |

`priority: <name>` labels describe impact. `status: <name>` labels describe why work is or is not moving.

## Triage

When triaging an issue:

1. Keep `needs-triage` until area and priority are clear.
2. Replace `priority: needs-priority` with exactly one concrete priority label.
3. Add the most specific accurate `area: <name>` labels.
4. If the issue cannot move yet, add `status: needs-info` or `status: blocked` and leave a comment naming the next action.

Do not use `status: stale` during initial triage. Stale means the issue was once actionable or under discussion, then went quiet.

## Blocked Issues

Use `status: blocked` only when the blocking condition is concrete. Good blocked comments name:

- the missing decision, upstream issue, reproduction, release artifact, or toolchain behavior
- who or what can unblock the issue
- what maintainers will do when the blocker clears

Keep the original priority unless the blocker changes user impact. For example, a release-blocking attestation failure can stay `priority: critical` while also being `status: blocked` if it is waiting on external infrastructure.

Review blocked issues before each release candidate. Close only when the underlying need is gone, duplicated elsewhere, or no longer fits the roadmap.

## Stale Issues

Use `status: stale` for inactive issues that need a freshness check before work resumes.

Suggested stale thresholds:

- 30 days without reporter response for `status: needs-info`
- 60 days without maintainer activity for `priority: low`
- 90 days without maintainer activity for `priority: medium`

Do not mark `priority: critical` or `priority: high` issues stale unless a maintainer explicitly confirms that the risk is no longer active.

When marking stale, leave a comment explaining what confirmation is needed. Remove `status: stale` as soon as a contributor provides current reproduction details, confirms continued demand, or links fresh evidence.

## Closing

Close an issue when:

- the fix shipped and validation is linked
- the issue is a duplicate and the canonical issue is linked
- the request is outside documented non-goals
- a stale issue has no response after a maintainer-requested follow-up window

When closing, leave the reason in the final comment. Do not close blocked issues just to keep the issue list short.
