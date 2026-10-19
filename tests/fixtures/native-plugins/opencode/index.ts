// Disposable v2 API candidate. Native loading and disposal remain unqualified.
import { Plugin } from "@opencode/plugin";
import { appendFileSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));
const config = JSON.parse(readFileSync(resolve(root, "fixture.json"), "utf8"));
const text = readFileSync(resolve(root, "rule.txt"), "utf8");

export default Plugin.define({
  id: `upskill-probe-${config.rule}`,
  async setup(ctx) {
    const registration = await ctx.session.hook("context", async (event) => {
      const session = await ctx.session.get({ sessionID: event.sessionID });
      if (resolve(session.location.directory) !== resolve(config.project) || event.agent !== config.agent) return;
      event.system.push({ type: "text", text });
      appendFileSync(config.trace, JSON.stringify({
        rule: config.rule, session: event.sessionID, source: "context",
      }) + "\n");
    });
    return () => registration.dispose();
  },
});
