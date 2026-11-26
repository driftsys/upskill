---
schema: 1
name: smoke-match-bash
description: Command hook on post-tool with a per-client matcher for bash
events:
  post-tool:
    matcher: { claude: "Bash", copilot: "bash" }
    run: match-bash.sh
    timeout: 10
---

## post-tool

Runs `match-bash.sh`, which appends one line to a matcher marker file.
