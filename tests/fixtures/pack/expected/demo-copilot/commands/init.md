---
description: Install the rules of the demo bundle into this repository
---

Run this command in the root directory of the repository. Then report its
output to the user.

```text
upskill add driftsys/demo@v1 demo --kind rule --project --copilot
```

If the command fails, report the error text. Do not run another command in its
place.

The command does not install the following. After the install, give this list
to the user:

- plugin `base-tool`
- plugin `cotool`: <https://example.com/cotool>
- plugin `manual`: <https://example.com/copilot-manual>
- MCP server `base-server`
- MCP server `srv`
