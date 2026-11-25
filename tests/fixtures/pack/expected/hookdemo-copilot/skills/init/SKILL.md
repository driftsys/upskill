---
name: init
description: Install the rules of the hookdemo bundle into this repository
disable-model-invocation: true
---

Run this command in the root directory of the repository. Then report its
output to the user.

```text
upskill add driftsys/demo@v1 hookdemo --kind rule --project --copilot
```

If the command fails, report the error text. Do not run another command in its
place.
