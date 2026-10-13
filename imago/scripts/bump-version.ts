import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const files = {
  pkg: "package.json",
  lock: "package-lock.json",
  cargoToml: "src-tauri/Cargo.toml",
  cargoLock: "src-tauri/Cargo.lock",
  tauri: "src-tauri/tauri.conf.json",
};

const args = process.argv.slice(2);
const doGit = args.includes("--git");
const arg = args.find((a) => !a.startsWith("--"));

if (!arg) {
  console.error("Usage: bun run bump <major|minor|patch|x.y.z> [--git]");
  process.exit(1);
}

const read = (p: string) => readFileSync(p, "utf8");

// --- work out the new version ---
const current: string = JSON.parse(read(files.pkg)).version;
let next = arg;
if (["major", "minor", "patch"].includes(arg)) {
  const [maj, min, pat] = current.split(".").map(Number);
  next =
    arg === "major" ? `${maj + 1}.0.0` :
    arg === "minor" ? `${maj}.${min + 1}.0` :
    `${maj}.${min}.${pat + 1}`;
}
if (!/^\d+\.\d+\.\d+([-+][\w.]+)?$/.test(next)) {
  console.error(`"${next}" is not a valid version`);
  process.exit(1);
}

// --- bail out early if the tag already exists ---
const git = (...a: string[]) => execFileSync("git", a, { stdio: "pipe" }).toString().trim();
if (doGit && git("tag", "--list", `v${next}`)) {
  console.error(`Tag v${next} already exists`);
  process.exit(1);
}

// --- JSON files (keeps indentation + trailing newline) ---
function editJson(path: string, fn: (j: any) => void) {
  const raw = read(path);
  const indent = raw.match(/^([ \t]+)"/m)?.[1] ?? 2;
  const json = JSON.parse(raw);
  fn(json);
  writeFileSync(path, JSON.stringify(json, null, indent) + (raw.endsWith("\n") ? "\n" : ""));
}

editJson(files.pkg, (j) => (j.version = next));
editJson(files.tauri, (j) => (j.version = next));
editJson(files.lock, (j) => {
  j.version = next;
  if (j.packages?.[""]) j.packages[""].version = next;
});

// --- Cargo.toml: first version under [package] ---
let toml = read(files.cargoToml);
const crateName = toml.match(/\[package\][\s\S]*?^name\s*=\s*"([^"]+)"/m)?.[1];
if (!crateName) throw new Error("Couldn't find crate name in Cargo.toml");
toml = toml.replace(/(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m, `$1${next}$2`);
writeFileSync(files.cargoToml, toml);

// --- Cargo.lock: the entry for this crate ---
let lock = read(files.cargoLock);
const re = new RegExp(`(name = "${crateName}"\\r?\\nversion = ")[^"]+(")`);
if (!re.test(lock)) throw new Error(`Couldn't find "${crateName}" in Cargo.lock`);
writeFileSync(files.cargoLock, lock.replace(re, `$1${next}$2`));

console.log(`${current} → ${next}`);

// --- optional: commit + tag ---
if (doGit) {
  const paths = Object.values(files);
  git("add", ...paths);
  // pass paths to commit so unrelated staged changes aren't swept in
  git("commit", "-m", `chore: bump version to ${next}`, "--", ...paths);
  git("tag", `v${next}`);
  console.log(`Committed and tagged v${next}`);
}