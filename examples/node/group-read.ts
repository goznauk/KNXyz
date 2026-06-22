/**
 * KNXyz example - group READ (DEFAULT-SAFE: dry-run).
 *
 * SAFETY: a `GroupValueRead` is an ACTIVE bus telegram (it solicits the bus), so
 * by default this performs NO bus I/O. It prints the read it WOULD perform and
 * exits WITHOUT opening a socket (it does not even load the native binding on the
 * default path). A real live read is advanced-only and ISOLATED-BUS-ONLY: it
 * requires `--live`, `KNXYZ_EXAMPLE_ALLOW_LIVE=1`, and explicit
 * `--gateway-host`/`--group-address`/`--dpt`; it refuses
 * documentation/placeholder hosts and refuses to run under CI. The live path
 * closes the tunnel in a `finally` block via the `client.close()` lifecycle
 * behavior. NEVER point this at a production or shared KNX bus. See
 * examples/README.md.
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
  const port = Number(arg("--port") ?? "3671");

  const armed =
    has("--live") &&
    process.env.KNXYZ_EXAMPLE_ALLOW_LIVE === "1" &&
    gatewayHost !== undefined &&
    groupAddress !== undefined &&
    dpt !== undefined;

  if (!armed) {
    // a documentation placeholder host (RFC 5737 TEST-NET); never contacted here
    const host = gatewayHost ?? "203.0.113.10";
    console.log(
      `DRY-RUN: would read GA=${groupAddress ?? "1/0/0"} DPT=${dpt ?? "9.001"} ` +
        `from ${host}:${port} (no connection made).`,
    );
    console.log(
      "For a REAL read on an ISOLATED test bus ONLY, pass --live " +
        "--gateway-host <host> --group-address <ga> --dpt 9.001 and set " +
        "KNXYZ_EXAMPLE_ALLOW_LIVE=1. NEVER target a production/shared bus.",
    );
    return;
  }

  // live path: advanced, isolated-bus-only, fail-closed
  if (isCi()) {
    throw new Error("refusing a live read under CI");
  }
  if (isPlaceholderHost(gatewayHost as string)) {
    throw new Error(
      "refusing a live read against a documentation/placeholder host; pass " +
        "--gateway-host for your own isolated test gateway",
    );
  }

  // load the native binding ONLY on the armed live path
  const { connectTunnel } = await import("@knxyz/knx");
  console.log(
    `LIVE: reading GA=${groupAddress} as DPT=${dpt} from ${gatewayHost} (ISOLATED test bus)...`,
  );
  const client = await connectTunnel({ host: gatewayHost, port });
  try {
    const value = await client.read(groupAddress as string, dpt as string);
    console.log(value);
  } finally {
    // orderly best-effort teardown
    await client.close();
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
