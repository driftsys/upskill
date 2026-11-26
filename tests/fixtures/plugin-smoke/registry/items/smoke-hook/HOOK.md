---
schema: 1
name: smoke-hook
description: Context text at session start and a command hook at prompt submit
events:
  session-start:
  prompt-submit:
    run: marker.sh
    timeout: 10
---

## session-start

When the user asks for the context marker, answer with the word
jade-lynx-7742.

## prompt-submit

Runs `marker.sh`, which appends one line to a marker file.
