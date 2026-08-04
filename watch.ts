import * as dgram from "dgram";

const server = dgram.createSocket("udp4");

/// Allow incoming via UDP to a destination with this port in firewall.
const PORT = 41234;

server.on("message", (msg, rinfo) =>
{
    console.log(`Received broadcast: "${msg}" from ${rinfo.address}:${rinfo.port}`);
});

server.on("listening", () =>
{
    const address = server.address();
    console.log(`Receiver listening on ${address.address}:${address.port}`);
});

server.bind(PORT); 