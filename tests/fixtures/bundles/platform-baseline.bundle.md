---
schema: 1
name: platform-baseline
description: Baseline rules, skills, and agents for all repositories
license: proprietary

items:
  rules:
    - api-conventions
    - license-awareness
  skills:
    - create-api-endpoint
  agents:
    - security-reviewer

metadata:
  version: "1.0.0"
  author: platform-dx
---

# Platform baseline

Bundles every fixture item under `tests/fixtures/`. Used by parse and
(eventually) install ATDD tests.
