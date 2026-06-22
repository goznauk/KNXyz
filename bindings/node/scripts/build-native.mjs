import { existsSync, readdirSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";
import { spawnSync } from "node:child_process";

const cargoDir = findCargoDir();
const env = { ...process.env };

if (cargoDir) {
  env.PATH = [cargoDir, env.PATH].filter(Boolean).join(delimiter);
}

const result = spawnSync("napi", ["build", "--platform", "--release"], {
  env,
  shell: process.platform === "win32",
  stdio: "inherit",
});

process.exit(result.status ?? 1);

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
