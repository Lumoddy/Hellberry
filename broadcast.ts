import * as dgram from "dgram";
import * as sp from "serialport";
import { BeadInterface } from "./bead/interface.ts";

const client = dgram.createSocket("udp4");

const BROADCAST_ADDR = "192.168.8.255";
const PORT = 41234;

client.bind(() =>
{
    client.setBroadcast(true); 

    sp.SerialPort.list()
        .then((x) => x.find((x) => x.manufacturer !== undefined && /Arduino/i.test(x.manufacturer)))
        .then((info) =>
        {
            if (info === undefined)
                throw new Error("Serial port not found");

            const port = new sp.SerialPort({ baudRate: 9600, path: info.path });

            port.on("open", () =>
            {
                const bead = new BeadInterface(port);

                console.log(`Serial port ${info.path} opened at ${port.baudRate} baud`);

                setTimeout(async () =>
                {
                    await bead.connect();

                    await bead.setPinMode("2", "digital-listen");

                    bead.on("pin-changed", (pin, power) =>
                    {
                        broadcast(power ? "bead-detected" : "bead-undetected");
                    });
                },
                    2000);
            });
        });
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
    });
}