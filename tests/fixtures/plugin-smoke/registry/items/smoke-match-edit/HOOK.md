---
schema: 1
name: smoke-match-edit
description: Command hook on post-tool with a per-client matcher for edit
events:
  post-tool:
    matcher: { claude: "Edit", copilot: "edit" }
    run: match-edit.sh
    timeout: 10
---

## post-tool

Runs `match-edit.sh`, which appends one line to a matcher marker file.
