# Copilot marketplace source classification check

Status: executed once on 2026-11-04 under the user's new diagnosis goal. The
scope was one offline setup and two read-only client queries in a fresh packet.
The earlier mutation approval was not reused. This grants no retry, installation
or qualification authority.

## Question and evidence

Determine whether the pinned Copilot marketplace inventory contains a formatted
source string that the contract collector rejects as a filesystem source. The
prior trial retained `outside_packet_source`, 113 stdout bytes and zero stderr
bytes, but discarded the scalar source. Its missing original value cannot be
recovered by a later observation. A reproduced formatted-source rejection would
establish a current cause consistent with the old trace; exact historical
identity would remain an inference. Static installed JavaScript alone does not
settle this question.

## Prepared packet

- Root: `/private/tmp/upskill-marketplace-source-check-ck88rz0x/packet`.
- Plan SHA-256:
  `ab277a41d6d32cb1173663fc0d32ce341b2929fd27a36fadbd8c78153f67767a`.
- Preparation SHA-256:
  `fccfd3eb321d1a659aa497399751b682167217f849e97f3d55083fe0450533e0`.

Preparation captures the current selected/resolved launcher, Node and platform
entrypoint paths, bytes, modes and digests. The strict read-only preflight
passed all nine fixture files. The plan contains the original eleven-command
contract for compiler/preflight compatibility, but this check may execute only
the explicit reduced sequence below. The new `--execute-marketplace-check` mode
selects that sequence in code. Fake-client tests pin both the library selection
and the example dispatch; neither can fall through to registration.

## Fixed execution envelope

Use the existing bounded runner, exact qualified roots, closed stdin and minimal
HOME/Copilot/cache/XDG/temp/Git environment from the contract collector. No
credentials or client options are inherited. Execute once and in order:

1. `/usr/bin/git` offline fixture setup with the existing fixed argv:
   `-c init.defaultBranch=main -c core.hooksPath=/dev/null -c init.templateDir= init --quiet --template= .`.
2. Pinned launcher argv: `--no-auto-update --version`.
3. Pinned launcher argv: `--no-auto-update plugin marketplace list --json`.

At most one setup and two client commands, twenty seconds and 1 MiB combined
output per command, zero model/provider requests and zero spending. Version must
match the exact retained 1.0.105 bytes. Stop on the first error, version or
input drift, incomplete transport, stderr, unknown schema or rejected source.
The packet is consumed on success or failure. No additional inventory,
registration, installation, removal, cleanup, retry or fallback is included.

Before launch, require the reviewed driver digest and unchanged current source
inputs, then let the preflight revalidate code assets, fixture files and roots.
Exclusively publish approval and execution records outside the packet; collector
Pending/result records bind each actual command before launch. Reaping children
and observing an empty group do not qualify escaped-worker quiescence.

## Projection and interpretation

Keep the original strict transport/stderr/schema validation before source
classification. Retain only fixed lexical source-category labels and sanitized
transport facts. Do not persist raw JSON, source strings, native names,
configuration, credentials, logs or sessions. Do not open source paths or strip
prefixes for admission. Known non-secret fixture snapshots remain permitted.

A `display_github`, `display_local` or `display_url` refusal confirms that a
formatted source reached the rejecting branch in this fresh diagnostic. A
non-display category falsifies that explanation for this observation. A version,
transport or schema refusal is inconclusive. No outcome grants trusted origin,
isolation, absence, cleanup or production admission. Retain the first result and
leave the packet intact.

## Observed result

The version matched the retained 1.0.105 bytes. The marketplace query exited
zero with complete output, zero stderr and 113 stdout bytes. Its source reached
the rejecting branch with `source_rejection_category: display_github` and
`reason: outside_packet_source`. This confirms a `GitHub:` display prefix was
treated as an invalid filesystem-source string in the fresh run.

The prior trace also had a matching version and a 113-byte marketplace response.
The reproduction directly establishes this failure mode and supports it as the
explanation for the prior rejection. Exact historical scalar identity remains
unprovable because that value was deliberately discarded. No outside-profile
read, trusted origin or actual external path is established by the failure.

Read-only verification checked three Pending/result digest links, exact selected
argv, all nine fixture bytes/modes and unchanged source/driver/native asset
pins. The packet is consumed and retained. There were no plugin inventory,
registration, installation, cleanup or model commands. Quiescence and production
admission remain unqualified. The sanitized
[summary](../../technotes/evidence/2026-11-04-copilot-marketplace-source-check/summary.json)
retains the causal category and transport facts; native source scalars remain
private and were not persisted.

The focused library and example tests passed and independent review found no
concrete bug in the restricted mode. Full verification encountered timing
failures in the Claude marketplace observer and native process tests. Both files
passed isolated reruns without source or limit changes; all 104 integration
targets were covered. Library tests, doctests, lint, docs and build checks
passed. The
[verification receipt](../../technotes/evidence/2026-11-04-copilot-marketplace-source-check/verification.json)
preserves the initial failures and distinct later outcomes rather than claiming
a clean standard full-suite run.
