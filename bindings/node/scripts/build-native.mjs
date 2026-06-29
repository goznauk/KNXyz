import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const cargoDir = findCargoDir();
const env = { ...process.env };
const declarationPath = fileURLToPath(new URL("../index.d.ts", import.meta.url));
const declarationBackup = existsSync(declarationPath)
  ? readFileSync(declarationPath)
  : undefined;

if (cargoDir) {
  env.PATH = [cargoDir, env.PATH].filter(Boolean).join(delimiter);
}

const result = spawnSync("napi", ["build", "--platform", "--release"], {
  env,
  shell: process.platform === "win32",
  stdio: "inherit",
});

const status = result.status ?? 1;
if (status === 0 && declarationBackup?.length) {
  if (!existsSync(declarationPath) || statSync(declarationPath).size === 0) {
    writeFileSync(declarationPath, declarationBackup);
  }
}

process.exit(status);

function findCargoDir() {
  if (spawnSync("cargo", ["--version"], { stdio: "ignore" }).status === 0) {
    return undefined;
  }

  const home = homedir();
  const candidates = [join(home, ".cargo", "bin")];
  const toolchains = join(home, ".rustup", "toolchains");

  if (existsSync(toolchains)) {
    for (const entry of readdirSync(toolchains)) {
      candidates.push(join(toolchains, entry, "bin"));
    }
  }

  return candidates.find((candidate) => existsSync(join(candidate, cargoName())));
}

function cargoName() {
  return process.platform === "win32" ? "cargo.exe" : "cargo";
}
