---
schema: 1
name: upskill-setup
description: Print the command that installs the upskill CLI and report upskill updates at session start
events:
  session-start:
    run: upskill-setup.sh
    timeout: 30
---

## session-start

Runs `upskill-setup.sh`. When the `upskill` CLI is not on `PATH`, the script
prints the install command once per day. When the CLI is present, the script
prints the output of `upskill update --notice`, if there is any. Copilot CLI
runs `upskill update --notice --repeat`, because it gives the line to the model
and does not show it to the user. The script always exits 0.
