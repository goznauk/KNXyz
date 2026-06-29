import {
  type DiscoverOptions,
  type Gateway,
  type TunnelClient,
  type TunnelOptions,
  connectTunnel,
  decodeDpt,
  discoverGateways,
  encodeDpt,
  formatGroupAddress,
  formatIndividualAddress,
  parseGroupAddress,
  parseIndividualAddress,
} from "@knxyz/knx";

const payload: Uint8Array = encodeDpt("9.001", 21.0);
const decoded: unknown = decodeDpt("9.001", payload);

const groupAddress: string = formatGroupAddress(parseGroupAddress("1/2/3"));
const individualAddress: string = formatIndividualAddress(parseIndividualAddress("1.1.4"));

const gateways: Promise<Gateway[]> = discoverGateways({
  timeoutMs: 100,
} satisfies DiscoverOptions);

const tunnel: Promise<TunnelClient> = connectTunnel({
  host: "knxip.example",
  port: 3671,
} satisfies TunnelOptions);

async function useTunnelClient(client: TunnelClient): Promise<void> {
  await client.write("1/2/3", 21.0, "9.001");
  const value: unknown = await client.read("1/2/3", "9.001", 1000);
  await client.close();
  await client.disconnect();
  void value;
}

void decoded;
void groupAddress;
void individualAddress;
void gateways;
void tunnel;
void useTunnelClient;
