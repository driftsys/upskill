---
schema: 1
name: create-api-endpoint
description: Use when scaffolding new REST API endpoints with Zod validation, following project conventions for handler structure, error handling, and test coverage
license: proprietary
metadata:
  version: "1.0.0"
  author: platform-api
---

## Create a new API endpoint

Follow these steps to scaffold a complete endpoint:

1. Create the handler file at `src/api/handlers/{resource}.ts`.
2. Define the request schema using Zod in `src/api/schemas/{resource}.ts`.
3. Register the route in `src/api/routes.ts`.
4. Create the test file at `src/api/__tests__/{resource}.test.ts`.

<!-- @client:claude -->

When creating files, use the Write tool for new files rather than Edit.

<!-- @endclient -->

## Validation pattern

All inputs MUST be validated against the Zod schema before processing.
See [validation patterns](./references/validation-patterns.md) for examples.
