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
  close(): Promise<void>;
  disconnect(): Promise<void>;
}

export function encodeDpt(dpt: string, value: unknown): Uint8Array;
export function decodeDpt(dpt: string, bytes: Uint8Array): unknown;
export function parseIndividualAddress(value: string): string;
export function formatIndividualAddress(value: string): string;
export function parseGroupAddress(value: string): string;
export function formatGroupAddress(value: string): string;
export function discoverGateways(options?: DiscoverOptions): Promise<Gateway[]>;
export function connectTunnel(options: TunnelOptions): Promise<TunnelClient>;
