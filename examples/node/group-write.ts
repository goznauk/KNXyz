/**
 * KNXyz example - group WRITE (DEFAULT-SAFE: dry-run).
 *
 * SAFETY: by default this performs NO bus I/O. It prints the telegram it WOULD
 * write and exits WITHOUT opening a socket (it does not even load the native
 * binding on the default path). A real live write is advanced-only and
 * ISOLATED-BUS-ONLY: it requires ALL of `--live`,
 * `KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1`, `--confirm ISOLATED_TEST_BUS_ONLY`, and
 * explicit `--gateway-host`/`--group-address`/`--dpt`/`--value`; it refuses
 * documentation hosts and refuses to run under CI. The live path closes the
 * tunnel in a `finally` block with `client.close()`. NEVER point this at a
 * production or shared KNX bus. Live writes require an explicit gateway host and
 * an opt-in environment variable. See examples/README.md.
 */

function arg(name: string): string | undefined {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

function has(name: string): boolean {
  return process.argv.includes(name);
}

function isCi(): boolean {
  const ci = process.env.CI ?? "";
  return ci !== "" && ci !== "0" && ci !== "false";
}

// RFC 5737 TEST-NET / RFC 3849 documentation ranges + symbolic tokens; refused live
function isPlaceholderHost(host: string): boolean {
  return (
    host.startsWith("192.0.2.") ||
    host.startsWith("198.51.100.") ||
    host.startsWith("203.0.113.") ||
    host.startsWith("2001:db8") ||
    host.includes("example") ||
    host.startsWith("<")
  );
}

async function main(): Promise<void> {
  const gatewayHost = arg("--gateway-host");
  const groupAddress = arg("--group-address");
  const dpt = arg("--dpt");
  const value = arg("--value");
  const port = Number(arg("--port") ?? "3671");

  // A live write also requires EVERY telegram parameter explicitly (no silent
  // default GA/DPT/value on the live path); any missing factor falls back to
  // dry-run (fail-closed).
  const armed =
    has("--live") &&
    process.env.KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE === "1" &&
    arg("--confirm") === "ISOLATED_TEST_BUS_ONLY" &&
    gatewayHost !== undefined &&
    groupAddress !== undefined &&
    dpt !== undefined &&
    value !== undefined;

  if (!armed) {
    // a documentation placeholder host (RFC 5737 TEST-NET); never contacted here
    const host = gatewayHost ?? "203.0.113.10";
    console.log(
      `DRY-RUN: would write GA=${groupAddress ?? "1/2/3"} DPT=${dpt ?? "1.001"} ` +
        `value=${value ?? "true"} to ${host}:${port} (no connection made).`,
    );
    console.log(
      "For a REAL write on an ISOLATED test bus ONLY, pass --live " +
        "--confirm ISOLATED_TEST_BUS_ONLY --gateway-host <host> --group-address <ga> " +
        "--dpt 1.001 --value true and set KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1. " +
        "NEVER target a production/shared bus.",
    );
    return;
  }

  // live path: advanced, isolated-bus-only, fail-closed
  if (isCi()) {
    throw new Error("refusing a live write under CI");
  }
  if (isPlaceholderHost(gatewayHost as string)) {
    throw new Error(
      "refusing a live write to a documentation host; pass " +
        "--gateway-host for your own isolated test gateway",
    );
  }

  // load the native binding ONLY on the armed live path
  const { connectTunnel } = await import("@knxyz/knx");
  console.log(
    `LIVE: writing ${value} to GA=${groupAddress} on ${gatewayHost} (ISOLATED test bus)...`,
  );
  const client = await connectTunnel({ host: gatewayHost, port });
  try {
    await client.write(groupAddress as string, value === "true", dpt as string);
    console.log("done");
  } finally {
    // orderly best-effort teardown, sends no write
    await client.close();
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
