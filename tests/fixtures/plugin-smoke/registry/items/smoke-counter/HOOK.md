---
schema: 1
name: smoke-counter
description: Count each session start outside the conversation
events:
  session-start:
    run: counter.sh
    timeout: 10
---

## session-start

Runs `counter.sh`, which appends one line to a counter file.
