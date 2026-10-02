import * as sp from "serialport";
import { BeadInterface } from "./bead/interface.ts";
import { SerialHandler } from "./serial_handler/interface.ts";

const serialHandler = new SerialHandler(
{
    condition: (info) => info.manufacturer !== undefined && /[Aa]rduino/.test(info.manufacturer),
    construct: BeadInterface,
    async created(handler)
    {
        handler.connect();

        handler.on("")
    },
});

setTimeout(() => serialHandler.update(), 1000);