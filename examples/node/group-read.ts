/**
 * KNXyz example: group read.
 *
 * The default run is a dry run. Live mode requires `--live`,
 * `KNXYZ_EXAMPLE_ALLOW_LIVE=1`, and explicit gateway, group address, and DPT
 * arguments. Placeholder hosts are rejected, and the live tunnel is closed in a
 * `finally` block. See examples/README.md.
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

// RFC 5737 TEST-NET / RFC 3849 documentation ranges and placeholder tokens.
function isPlaceholderHost(host: string): boolean {
  const normalized = host.startsWith("[") ? host.slice(1) : host;
  return (
    normalized.startsWith("192.0.2.") ||
    normalized.startsWith("198.51.100.") ||
    normalized.startsWith("203.0.113.") ||
    normalized.startsWith("2001:db8") ||
    normalized.includes("example") ||
    normalized.startsWith("<")
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
    // Dry-run output uses a documentation placeholder host.
    const host = gatewayHost ?? "203.0.113.10";
    console.log(
      `DRY-RUN: would read GA=${groupAddress ?? "1/0/0"} DPT=${dpt ?? "9.001"} ` +
        `from ${host}:${port} (no connection made).`,
    );
    console.log(
      "For a live read, pass --live " +
        "--gateway-host <host> --group-address <ga> --dpt 9.001 and set " +
        "KNXYZ_EXAMPLE_ALLOW_LIVE=1.",
    );
    return;
  }

  // Live path: require explicit arguments, the environment variable, and non-placeholder host.
  if (isCi()) {
    throw new Error("refusing a live read under CI");
  }
  if (isPlaceholderHost(gatewayHost as string)) {
    throw new Error(
      "refusing a live read against a documentation/placeholder host; pass " +
        "--gateway-host for your gateway",
    );
  }

  // Load the native binding only on the armed live path.
  const { connectTunnel } = await import("@knxyz/knx");
  console.log(`LIVE: reading GA=${groupAddress} as DPT=${dpt} from ${gatewayHost}...`);
  const client = await connectTunnel({ host: gatewayHost, port });
  try {
    const value = await client.read(groupAddress as string, dpt as string);
    console.log(value);
  } finally {
    // Close the tunnel regardless of the read result.
    await client.close();
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
