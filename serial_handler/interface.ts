import * as sp from "serialport";
import * as os from "os";

export interface SerialHandlerOptions<H>
{
    condition?: (this: SerialHandler<H>, portInfo: PortInfo) => boolean,
    create?: (this: SerialHandler<H>, serialPort: sp.SerialPort) => H | null | undefined,
    construct?: new (options: { serialPort: sp.SerialPort, source: sp.SerialPort }) => H,
    created?: (this: SerialHandler<H>, handler: H, serialPort: sp.SerialPort) => void,
    baudRate?: number,
}

export type PortInfo = typeof sp.SerialPort.list extends () => Promise<(infer T)[]> ? T : never;

export class SerialHandler<H>
{
    #serialPorts: Map<string, H> = new Map();
    #condition: ((this: SerialHandler<H>, portInfo: PortInfo) => boolean) | undefined;
    #create: ((this: SerialHandler<H>, serialPort: sp.SerialPort) => H | null | undefined) | undefined;
    #construct: (new (options: { serialPort: sp.SerialPort, source: sp.SerialPort }) => H) | undefined;
    #created: ((this: SerialHandler<H>, handler: H, serialPort: sp.SerialPort) => void) | undefined;
    #baudRate: number;

    constructor(options: SerialHandlerOptions<H>)
    {
        this.#condition = options.condition;
        this.#create = options.create;
        this.#construct = options.construct;
        this.#created = options.created;
        this.#baudRate = options.baudRate ?? 9600;
    }

    async update(): Promise<void>
    {
        if (!(this instanceof SerialHandler))
            throw new TypeError(
                `'update' called on an object that does not implement interface SerialHandler.`);

        const portInfos = await sp.SerialPort.list();

        for (const portInfo of portInfos)
        {
            const path = portInfo.path;

            if (this.#serialPorts.has(path)
                || this.#condition === undefined
                || !this.#condition(portInfo))
                continue;

            const serialPort = new sp.SerialPort({ baudRate: this.#baudRate, path });

            const created =
                this.#create !== undefined ? this.#create(serialPort) :
                this.#construct !== undefined ? new this.#construct({ serialPort, source: serialPort }) :
                undefined;

            if (created != null)
            {
                this.#serialPorts.set(path, created);

                try { this.#created?.(created, serialPort) }
                finally
                {
                    serialPort.once("close", () => this.#serialPorts.delete(path));
                }
            }
        }
    }
}