import { spawnSync } from "node:child_process";
import { copyFileSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// The supplied icon-1.png is the source of truth. Requires Python + Pillow.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "src-tauri/icons");
const staging = mkdtempSync(join(tmpdir(), "countdown-icons-"));
function generate(source, destination, sizes = []) {
  const args = ["tauri", "icon", source, "--output", destination];
  for (const size of sizes) args.push("--png", String(size));
  const result = spawnSync("cargo", args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Icon generation failed (${result.status})`);
}
try {
  const prepared = spawnSync("python", ["scripts/prepare-icon.py"], { cwd: root, stdio: "inherit" });
  if (prepared.error) throw prepared.error;
  if (prepared.status !== 0) throw new Error(`Source preparation failed (${prepared.status})`);
  generate("src/assets/app-icon.png", staging);
  // Tauri also generates mobile assets; this desktop project needs only these files.
  for (const entry of readdirSync(staging, { withFileTypes: true })) {
    if (entry.isFile() && /\.(png|ico|icns)$/.test(entry.name)) {
      copyFileSync(join(staging, entry.name), join(output, entry.name));
    }
  }
} finally {
  rmSync(staging, { recursive: true, force: true });
}
