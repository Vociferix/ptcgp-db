# Always re-check the current branch before committing

In the pack-point-suggestions session I opened PR #65 from `feature-pack-point-suggestions`, the
user merged it and switched the working tree to `master`, and then a follow-up request came in.
I committed the follow-up work and pushed it **directly to master**, bypassing the project's
branch-per-task PR process.

Fix used (user approved the force-push explicitly): save the commit on a local branch first,
`git reset --hard <previous>`, `git push --force-with-lease origin master`, then push the branch
and open the PR.

## Rule

`git status` / `git branch --show-current` **before every commit**, not just at the start of a
session. A merged PR means the user has very likely moved back to `master`. The start-of-session
git status in the system prompt is a snapshot and goes stale the moment the user merges.
