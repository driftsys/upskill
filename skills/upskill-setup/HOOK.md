---
schema: 1
name: upskill-setup
description: Offer to install the upskill CLI and report upskill updates at session start
events:
  session-start:
    run: upskill-setup.sh
    timeout: 30
---

## session-start

Runs `upskill-setup.sh`. When the `upskill` CLI is not on `PATH`, the script
prints the install command once per day. When the CLI is present, the script
prints the output of `upskill update --notice`, if there is any. The script
always exits 0.
