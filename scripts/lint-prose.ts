import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// Lint tracked and new Markdown, including hidden agent instructions. Fail on
// warnings too; Vale's normal exit status only fails on errors.
const cwd = fileURLToPath(new URL("../", import.meta.url));
process.chdir(cwd);

const config = readFileSync(".vale.ini", "utf8");
const missing = [...config.matchAll(/^([\w-]+)\.([\w-]+)\s*=\s*(?:YES|warning|error|suggestion)\s*$/gm)]
  .map(([, style, rule]) => `.vale/styles/${style}/${rule}.yml`)
  .filter((path) => !existsSync(path));
if (missing.length) {
  console.error(`Missing Vale rules:\n${missing.join("\n")}\nRun vale sync --no-global first.`);
  process.exit(1);
}

function run(command: string[]) {
  try {
    return Bun.spawnSync(command, { cwd, stdout: "pipe", stderr: "pipe" });
  } catch (error) {
    console.error(`Could not run ${command[0]}: ${error instanceof Error ? error.message : error}`);
    process.exit(1);
  }
}

const listed = run(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "*.md", "*.mdx"]);
if (listed.exitCode !== 0) {
  console.error(listed.stderr.toString());
  process.exit(listed.exitCode || 1);
}
const files = [...new Set(listed.stdout.toString().split("\0").filter((path) => path && existsSync(path)))].sort();
if (!files.length) {
  console.error("No Markdown files found.");
  process.exit(1);
}

const result = run(["vale", "--no-global", "--output=JSON", ...files]);
if (result.exitCode !== 0) {
  console.error(result.stderr.toString() || result.stdout.toString());
  process.exit(result.exitCode || 1);
}

type Alert = { Line: number; Span: [number, number]; Severity: string; Message: string; Check: string };
let findings: Record<string, Alert[]>;
try {
  findings = JSON.parse(result.stdout.toString());
} catch {
  console.error("Vale returned unreadable output:", result.stdout.toString(), result.stderr.toString());
  process.exit(1);
}
let count = 0;
for (const [path, alerts] of Object.entries(findings)) {
  for (const alert of alerts) {
    console.error(`${path}:${alert.Line}:${alert.Span[0]}: ${alert.Severity}: ${alert.Message} (${alert.Check})`);
    count++;
  }
}
console.log(`${files.length} Markdown files checked; ${count} Vale alerts.`);
process.exit(count ? 1 : 0);
