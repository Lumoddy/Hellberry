import * as dgram from "dgram";
import * as sp from "serialport";
import { BeadInterface } from "./bead/interface.ts";

const server = dgram.createSocket("udp4");

/// Allow incoming via UDP to a destination with this port in firewall.
const PORT = 41234;

server.bind(PORT, () =>
{
    const address = server.address();
    console.log(`Receiver listening on ${address.address}:${address.port}`);

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

                    await bead.setPinMode("3", "digital-output");

                    server.on("message", (message, rinfo) =>
                    {
                        const string = message.toString("ascii");

                        switch (string)
                        {
                            case "bead-detected":
                            {
                                bead.setPinPower("3", true);
                                break;
                            }
                            case "bead-undetected":
                            {
                                bead.setPinPower("3", false);
                                break;
                            }
                            default:
                            {
                                console.warn(`Received unknown broadcast: "${string}" from ${rinfo.address}:${rinfo.port}`);
                                break;
                            }
                        }
                    });
                },
                    2000);
            });
        });
});