# Gate 6 native instruction qualification

This bounded qualification slice derives from the existing M1 owned lifecycle
design and M1-B implementation contract. Delivery task: #319. Related debt:
#316. It does not activate production schema-2 commands.

The opt-in qualification harness must exercise fresh native client sessions in
disposable project fixtures. A canonical instruction positive control and a
sibling `.upskill-stage-<nonce>` negative fixture use an unpredictable marker
that is absent from the question. Qualification requires the positive response
to match that marker exactly and the negative to match an independent absence
response exactly. Failures, timeouts, unavailable clients, authentication errors
and unexpected responses must remain distinguishable from qualification.

Default instruction discovery and configured globs or imports must be recorded
as separate cases. Tool-based reads cannot establish automatic discovery.
Evidence must identify the client version, operating system, scope, relevant
configuration, commands and sanitized outcomes. A CLI probe does not qualify an
IDE or a different platform. User roots that cannot be safely isolated remain
explicitly unqualified; do not alter real user instructions to obtain a result.

Never read or copy authentication files into fixtures or evidence. Use the
client's normal authentication, disable tools where supported and bound session
runtime. Raw arbitrary client output must not become tracked documentation.
Never infer staging-file ownership or cleanup authority from its filename.

The resulting durable record must explain precisely what #316 evidence proves,
what it does not prove, and which Gate 6 requirements remain before activation.
The four existing default surfaces and production schema-1 behavior are
unchanged by this slice. All current item kinds, complete conflict aggregation,
Codex selection and qualified source-backed instruction adapters remain separate
activation requirements.
