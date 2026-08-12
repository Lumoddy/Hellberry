import * as sp from "serialport";
import { BeadInterface } from "./bead/interface.ts";

sp.SerialPort.list()
    // .then((x) => void console.log(x))
    .then((x) => x.find((x) => x.serialNumber === "34333323832351B0B241"))
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
                    console.log(pin, power);
                });

                port.on("error", (err) =>
                {
                    console.error("Serial port error:", err);
                });
            },
                2000);
        });
    });