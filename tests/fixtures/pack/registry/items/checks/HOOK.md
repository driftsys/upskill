---
schema: 1
name: edit-checks
description: Check each edited file
events:
  session-start:
  post-tool:
    matcher: { claude: "Edit|Write", copilot: edit }
    run: check.sh
    timeout: 10
---

## session-start

Run the checks before you commit.

## post-tool

Runs `check.sh` on the edited file.
