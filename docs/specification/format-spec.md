# Portable format compatibility reference

The canonical [content format](../topics/0001-content-format.md) retains its
Draft status. Existing section links are preserved below; edit the concern
reference rather than this compatibility page.

<a id="portable-format-for-ai-assistance-content"></a>

- [Portable Format for AI-Assistance Content](../topics/0001-content-format.md#portable-format-for-ai-assistance-content)

<a id="abstract"></a>

- [Abstract](../topics/0001-content-format.md#abstract)

<a id="audience"></a>

- [Audience](../topics/0001-content-format.md#audience)

<a id="conformance-keywords"></a>

- [Conformance keywords](../topics/0001-content-format.md#conformance-keywords)

<a id="client-name-conventions"></a>

- [Client-name conventions](../topics/0001-content-format.md#client-name-conventions)

<a id="1-terminology"></a>

- [1. Terminology](../topics/0001-content-format.md#1-terminology)

<a id="2-file-layout"></a>

- [2. File layout](../topics/0001-content-format.md#2-file-layout)

<a id="21-item-directory-structure"></a>

- [2.1 Item directory structure](../topics/0001-content-format.md#21-item-directory-structure)

<a id="22-bundle-files"></a>

- [2.2 Bundle files](../topics/0001-content-format.md#22-bundle-files)

<a id="23-per-client-override-files"></a>

- [2.3 Per-client override files](../topics/0001-content-format.md#23-per-client-override-files)

<a id="24-supporting-resources"></a>

- [2.4 Supporting resources](../topics/0001-content-format.md#24-supporting-resources)

<a id="3-frontmatter-schema"></a>

- [3. Frontmatter schema](../topics/0001-content-format.md#3-frontmatter-schema)

<a id="31-common-fields-all-kinds-and-bundles"></a>

- [3.1 Common fields (all kinds and bundles)](../topics/0001-content-format.md#31-common-fields-all-kinds-and-bundles)

<a id="32-rule-specific-fields"></a>

- [3.2 Rule-specific fields](../topics/0001-content-format.md#32-rule-specific-fields)

<a id="33-skill-specific-fields"></a>

- [3.3 Skill-specific fields](../topics/0001-content-format.md#33-skill-specific-fields)

<a id="34-agent-specific-fields"></a>

- [3.4 Agent-specific fields](../topics/0001-content-format.md#34-agent-specific-fields)

<a id="35-client-specific-passthrough-blocks"></a>

- [3.5 Client-specific passthrough blocks](../topics/0001-content-format.md#35-client-specific-passthrough-blocks)

<a id="36-metadata-block"></a>

- [3.6 Metadata block](../topics/0001-content-format.md#36-metadata-block)

<a id="37-bundle-schema"></a>

- [3.7 Bundle schema](../topics/0001-content-format.md#37-bundle-schema)

<a id="plugins-map"></a>

- [`plugins` map](../topics/0001-content-format.md#plugins-map)

<a id="mcps-map"></a>

- [`mcps` map](../topics/0001-content-format.md#mcps-map)

<a id="item-requires-resolution"></a>

- [Item `requires` resolution](../topics/0001-content-format.md#item-requires-resolution)

<a id="38-frontmatter-canonicalisation"></a>

- [3.8 Frontmatter canonicalisation](../topics/0001-content-format.md#38-frontmatter-canonicalisation)

<a id="4-capability-level-tool-vocabulary"></a>

- [4. Capability-level tool vocabulary](../topics/0001-content-format.md#4-capability-level-tool-vocabulary)

<a id="5-body-content-conventions"></a>

- [5. Body content conventions](../topics/0001-content-format.md#5-body-content-conventions)

<a id="51-heading-structure"></a>

- [5.1 Heading structure](../topics/0001-content-format.md#51-heading-structure)

<a id="52-fenced-code-blocks"></a>

- [5.2 Fenced code blocks](../topics/0001-content-format.md#52-fenced-code-blocks)

<a id="53-tool-references-in-prose"></a>

- [5.3 Tool references in prose](../topics/0001-content-format.md#53-tool-references-in-prose)

<a id="54-file-references"></a>

- [5.4 File references](../topics/0001-content-format.md#54-file-references)

<a id="55-prohibited-constructs-in-canonical-bodies"></a>

- [5.5 Prohibited constructs in canonical bodies](../topics/0001-content-format.md#55-prohibited-constructs-in-canonical-bodies)

<a id="6-conditional-content-directives"></a>

- [6. Conditional content directives](../topics/0001-content-format.md#6-conditional-content-directives)

<a id="61-syntax"></a>

- [6.1 Syntax](../topics/0001-content-format.md#61-syntax)

<a id="62-forms"></a>

- [6.2 Forms](../topics/0001-content-format.md#62-forms)

<a id="63-processing-rules"></a>

- [6.3 Processing rules](../topics/0001-content-format.md#63-processing-rules)

<a id="64-interaction-with-override-files"></a>

- [6.4 Interaction with override files](../topics/0001-content-format.md#64-interaction-with-override-files)

<a id="7-generation-client-specific-output"></a>

- [7. Generation: client-specific output](../topics/0001-content-format.md#7-generation-client-specific-output)

<a id="71-claude-code"></a>

- [7.1 Claude Code](../topics/0001-content-format.md#71-claude-code)

<a id="72-github-copilot"></a>

- [7.2 GitHub Copilot](../topics/0001-content-format.md#72-github-copilot)

<a id="73-opencode"></a>

- [7.3 opencode](../topics/0001-content-format.md#73-opencode)

<a id="74-formatting-guarantee"></a>

- [7.4 Formatting guarantee](../topics/0001-content-format.md#74-formatting-guarantee)

<a id="8-schema-versioning"></a>

- [8. Schema versioning](../topics/0001-content-format.md#8-schema-versioning)

<a id="81-rules-for-implementations"></a>

- [8.1 Rules for implementations](../topics/0001-content-format.md#81-rules-for-implementations)

<a id="82-rules-for-specification-evolution"></a>

- [8.2 Rules for specification evolution](../topics/0001-content-format.md#82-rules-for-specification-evolution)

<a id="9-conformance"></a>

- [9. Conformance](../topics/0001-content-format.md#9-conformance)

<a id="10-non-goals"></a>

- [10. Non-goals](../topics/0001-content-format.md#10-non-goals)

<a id="11-open-questions-for-future-versions"></a>

- [11. Open questions for future versions](../topics/0001-content-format.md#11-open-questions-for-future-versions)

<a id="appendix-a-complete-frontmatter-examples"></a>

- [Appendix A: Complete frontmatter examples](../topics/0001-content-format.md#appendix-a-complete-frontmatter-examples)

<a id="a1-rule--always-on-all-clients"></a>

- [A.1 Rule — always-on, all clients](../topics/0001-content-format.md#a1-rule--always-on-all-clients)

<a id="a2-rule--path-scoped-with-github-copilot-passthrough"></a>

- [A.2 Rule — path-scoped, with GitHub Copilot passthrough](../topics/0001-content-format.md#a2-rule--path-scoped-with-github-copilot-passthrough)

<a id="a3-skill--on-demand-with-directive"></a>

- [A.3 Skill — on-demand, with directive](../topics/0001-content-format.md#a3-skill--on-demand-with-directive)

<a id="a4-agent--subagent-with-tools-and-preloaded-skill"></a>

- [A.4 Agent — subagent with tools and preloaded skill](../topics/0001-content-format.md#a4-agent--subagent-with-tools-and-preloaded-skill)

<a id="a5-bundle--with-dependency"></a>

- [A.5 Bundle — with dependency](../topics/0001-content-format.md#a5-bundle--with-dependency)

<a id="a6-agent--with-per-client-override-file"></a>

- [A.6 Agent — with per-client override file](../topics/0001-content-format.md#a6-agent--with-per-client-override-file)

<a id="appendix-b-client-frontmatter-mapping-reference"></a>

- [Appendix B: Client frontmatter mapping reference](../topics/0001-content-format.md#appendix-b-client-frontmatter-mapping-reference)

<a id="appendix-c-directive-quick-reference"></a>

- [Appendix C: Directive quick reference](../topics/0001-content-format.md#appendix-c-directive-quick-reference)

<a id="appendix-d-recommended-source-registry-layout"></a>

- [Appendix D: Recommended source-registry layout](../topics/0001-content-format.md#appendix-d-recommended-source-registry-layout)
