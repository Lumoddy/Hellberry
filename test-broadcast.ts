import * as dgram from "dgram";
import { BeadInterface } from "./bead/interface.ts";

const client = dgram.createSocket("udp4");

const BROADCAST_ADDR = "192.168.8.255";
const PORT = 41234;

client.bind(() =>
{
    client.setBroadcast(true); 

    broadcast("bead-undetected");
});

function broadcast(message: string)
{
    const buffer = Buffer.from(message, "ascii");

    client.send(buffer, 0, buffer.length, PORT, BROADCAST_ADDR, (err) =>
    {
        if (err)
            console.error("Failed to send broadcast:", err);
        else
            console.log(`Broadcast message sent to ${BROADCAST_ADDR}:${PORT}`);

        client.close();
    });
}