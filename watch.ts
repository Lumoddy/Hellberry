import * as dgram from "dgram";
import * as sp from "serialport";
import { unescapeByte, parsePacket, IncomingPacket, START } from "./packet/incoming.js";
import { encodePacket, OutgoingPacket } from "./packet/outgoing.js";

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

    sp.SerialPort.list()
        .then((x) => x.find((x) => x.serialNumber === "34333323832351C052F1"))
        .then((info) =>
        {
            if (info === undefined)
                throw new Error("Serial port not found");

            const port = new sp.SerialPort({ baudRate: 9600, path: info.path });

            port.on("open", () =>
            {
                console.log(`Serial port ${info.path} opened at ${port.baudRate} baud`);

                setTimeout(() =>
                {
                    let unescape = unescapeByte();
                    unescape.next();
                    let parser = parsePacket(false);
                    parser.next();

                    const instance = serialInterface((packet) =>
                    {
                        console.log(encodePacket(packet));

                        port.write(encodePacket(packet), (err) =>
                        {
                            if (err)
                                console.error("Error writing to serial port:", err);
                            else
                                console.log(`Send '${packet.type}'`);
                        });
                    });
                    instance.next();

                    port.on("data", (data) =>
                    {
                        console.log(data);

                        for (const byte of data)
                        {
                            const { done: unescapedDone, value: unescapedByte } = unescape.next(byte);
                            if (!unescapedDone) continue;
                            unescape = unescapeByte();
                            unescape.next();

                            if (unescapedByte === START)
                            {
                                parser = parsePacket(false);
                                parser.next();
                            }
                            else
                            {
                                const { done: packetDone, value: packet } = parser.next(unescapedByte);
                                if (!packetDone) continue;
                                parser = parsePacket(false);
                                parser.next();

                                instance.next(packet);
                            }
                        }
                    });

                    port.on("error", (err) =>
                    {
                        console.error("Serial port error:", err);
                    });
                },
                    2000);
            });
        });
});

server.bind(PORT);

function* serialInterface(send: (packet: OutgoingPacket) => void)
    : Generator<undefined, never, IncomingPacket>
{
    send({ type: "get-config" });

    const config = (yield);
    if (config.type !== "whole-config")
        throw new Error("Failed to initialize interface");

    console.log("config", config);

    while (true)
    {
        console.log(yield);

        // switch (packet.type)
        // {
        //     case ""
        // }
    }
}