import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

export interface DiscoverOptions {
  bind?: string;
  target?: string;
  timeoutMs?: number;
}

export interface Gateway {
  controlEndpoint: string;
  receivedFrom: string;
  serviceFamilies: Array<{ id: number; version: number }>;
}

export interface TunnelOptions {
  host?: string;
  port?: number;
  target?: string;
  bind?: string;
  controlEndpoint?: string;
  dataEndpoint?: string;
  ackTimeoutMs?: number;
}

export interface TunnelClient {
  write(groupAddress: string, value: unknown, dpt: string): Promise<void>;
  read(groupAddress: string, dpt: string, timeoutMs?: number): Promise<unknown>;
  /**
   * Best-effort orderly disconnect; idempotent. Sends a KNXnet/IP
   * DISCONNECT_REQUEST and releases the connection. It sends no group write and
   * never reconnects, and it tolerates a silent gateway (it does not throw on
   * teardown).
   */
  close(): Promise<void>;
  /** Alias for {@link TunnelClient.close}. */
  disconnect(): Promise<void>;
}

interface NativeTunnelClient {
  write(groupAddress: string, dpt: string, valueJson: string): Promise<void>;
  read(groupAddress: string, dpt: string, timeoutMs: number): Promise<string>;
  close(): Promise<void>;
}

interface NativeBinding {
  encodeDptJson(dpt: string, valueJson: string): Uint8Array | Error;
  decodeDptJson(dpt: string, bytes: Uint8Array): string | Error;
  parseIndividualAddress(value: string): string;
  formatIndividualAddress(value: string): string;
  parseGroupAddress(value: string): string;
  formatGroupAddress(value: string): string;
  discoverGatewaysJson(optionsJson: string): Promise<string>;
  connectTunnelNative(optionsJson: string): Promise<NativeTunnelClient>;
}

const native = loadNative();

export function encodeDpt(dpt: string, value: unknown): Uint8Array {
  return unwrapNative(native.encodeDptJson(dpt, JSON.stringify(value)));
}

export function decodeDpt(dpt: string, bytes: Uint8Array): unknown {
  return JSON.parse(unwrapNative(native.decodeDptJson(dpt, bytes)));
}

export function parseIndividualAddress(value: string): string {
  return native.parseIndividualAddress(value);
}

export function formatIndividualAddress(value: string): string {
  return native.formatIndividualAddress(value);
}

export function parseGroupAddress(value: string): string {
  return native.parseGroupAddress(value);
}

export function formatGroupAddress(value: string): string {
  return native.formatGroupAddress(value);
}

export async function discoverGateways(options: DiscoverOptions = {}): Promise<Gateway[]> {
  return JSON.parse(await native.discoverGatewaysJson(JSON.stringify(options)));
}

export async function connectTunnel(options: TunnelOptions): Promise<TunnelClient> {
  const client = await native.connectTunnelNative(JSON.stringify(options));

  return {
    write(groupAddress: string, value: unknown, dpt: string): Promise<void> {
      return client.write(groupAddress, dpt, JSON.stringify(value));
    },
    async read(groupAddress: string, dpt: string, timeoutMs = 3000): Promise<unknown> {
      return JSON.parse(await client.read(groupAddress, dpt, timeoutMs));
    },
    close(): Promise<void> {
      return client.close();
    },
    disconnect(): Promise<void> {
      return client.close();
    },
  };
}

function loadNative(): NativeBinding {
  const require = createRequire(import.meta.url);
  const candidates = [
    "../index.linux-x64-gnu.node",
    "../index.linux-x64-musl.node",
    "../index.darwin-arm64.node",
    "../index.darwin-x64.node",
    "../index.win32-x64-msvc.node",
    // current Cargo crate name (knxyz-node) addon artifacts
    "../knxyz-node.linux-x64-gnu.node",
    "../knxyz-node.linux-x64-musl.node",
    "../knxyz-node.darwin-arm64.node",
    "../knxyz-node.darwin-x64.node",
    "../knxyz-node.win32-x64-msvc.node",
  ];

  for (const candidate of candidates) {
    const path = fileURLToPath(new URL(candidate, import.meta.url));
    if (existsSync(path)) {
      return require(path) as NativeBinding;
    }
  }

  throw new Error("native KNXyz Node binding has not been built");
}

function unwrapNative<T>(value: T | Error): T {
  if (value instanceof Error) {
    throw value;
  }

  return value;
}
