import * as dgram from "dgram";

const client = dgram.createSocket("udp4");

const BROADCAST_ADDR = "192.168.8.255";
const PORT = 41234;
const message = Buffer.from("Hello LAN! This is a Node.js broadcast.");

client.bind(() =>
{
    client.setBroadcast(true); 

    client.send(message, 0, message.length, PORT, BROADCAST_ADDR, (err) =>
    {
        if (err)
            console.error("Failed to send broadcast:", err);
        else
            console.log(`Broadcast message sent to ${BROADCAST_ADDR}:${PORT} from ${client.address().address}:${client.address().port}`);

        client.close();
    });
});