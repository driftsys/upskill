---
schema: 1
name: api-conventions
description: Use when writing or modifying API handler files to ensure consistent request validation, error formatting, and response shape
license: proprietary
scope:
  paths:
    - "src/api/**/*.ts"
    - "src/handlers/**/*.ts"
copilot:
  excludeAgent: code-review
metadata:
  version: "2.1.0"
  author: platform-api
---

## API handler conventions

- All endpoints include input validation using the project's schema library.
- Use the standard error response format defined in `src/api/errors.ts`.
- Never return raw database errors to the client.
