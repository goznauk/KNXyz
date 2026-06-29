/**
 * KNXyz example: boolean group write, DPT 1.001.
 *
 * The default run is a dry run. Live mode requires `--live`,
 * `KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1`, the confirmation flag, and all telegram
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

function parseBooleanWriteValue(dpt: string, value: string): boolean {
  if (dpt !== "1.001") {
    throw new Error(`this example demonstrates DPT 1.001 boolean writes; got ${dpt}`);
  }

  const lowered = value.toLowerCase();
  if (lowered === "true") {
    return true;
  }
  if (lowered === "false") {
    return false;
  }

  throw new Error(`this example accepts only --value true or --value false; got ${value}`);
}

async function main(): Promise<void> {
  const gatewayHost = arg("--gateway-host");
  const groupAddress = arg("--group-address");
  const dpt = arg("--dpt");
  const value = arg("--value");
  const port = Number(arg("--port") ?? "3671");

  // Live writes require every telegram parameter explicitly; otherwise the
  // example stays on the dry-run path.
  const armed =
    has("--live") &&
    process.env.KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE === "1" &&
    arg("--confirm") === "ISOLATED_TEST_BUS_ONLY" &&
    gatewayHost !== undefined &&
    groupAddress !== undefined &&
    dpt !== undefined &&
    value !== undefined;

  if (!armed) {
    // Dry-run output uses a documentation placeholder host.
    const host = gatewayHost ?? "203.0.113.10";
    console.log(
      `DRY-RUN: would write GA=${groupAddress ?? "1/2/3"} DPT=${dpt ?? "1.001"} ` +
        `value=${value ?? "true"} to ${host}:${port} (no connection made).`,
    );
    console.log(
      "For a live write, pass --live " +
        "--confirm ISOLATED_TEST_BUS_ONLY --gateway-host <host> --group-address <ga> " +
        "--dpt 1.001 --value true and set KNXYZ_EXAMPLE_ALLOW_LIVE_WRITE=1. " +
        "The confirmation flag is required for write examples.",
    );
    return;
  }

  // Live path: require explicit arguments, confirmation, environment variable,
  // and non-placeholder host.
  if (isCi()) {
    throw new Error("refusing a live write under CI");
  }
  if (isPlaceholderHost(gatewayHost as string)) {
    throw new Error(
      "refusing a live write to a documentation host; pass " +
        "--gateway-host for your gateway",
    );
  }

  const parsedValue = parseBooleanWriteValue(dpt as string, value as string);

  // Load the native binding only on the armed live path.
  const { connectTunnel } = await import("@knxyz/knx");
  console.log(
    `LIVE: writing ${parsedValue} as DPT=${dpt} to GA=${groupAddress} on ${gatewayHost}...`,
  );
  const client = await connectTunnel({ host: gatewayHost, port });
  try {
    await client.write(groupAddress as string, parsedValue, dpt as string);
    console.log("done");
  } finally {
    // Close the tunnel regardless of the write result.
    await client.close();
  }
}

main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
