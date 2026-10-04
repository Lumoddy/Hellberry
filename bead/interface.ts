import { EventEmitter } from "events";
import { deserializePacket, RESET, type IncomingPacket } from "./incoming.ts";
import { serialize } from "./outgoing.ts";

export type PinMode =
    | "digital-input"
    | "digital-listen"
    | "digital-output"
    | "analog-input"
    | "analog-output";

export interface BeadConfig
{
    name: string,
    pins: BeadPinConfig[],
}

export interface BeadPinConfig
{
    name: string,
    supportsDigitalInput: boolean,
    supportsDigitalOutput: boolean,
    supportsAnalogInput: boolean,
    supportsAnalogOutput: boolean,
}

export interface BeadInterfaceEventMap
{
    "packet": [packet: IncomingPacket],
    "packet-error": [error: SyntaxError],
    "pin-changed": [pin: number, name: string, power: number],
    "reset": [],
}

export interface BeadInterfaceOptions
{
    readonly source: BeadInterfaceSource,
}

export interface BeadInterfaceSource
{
    write(data: Buffer<ArrayBuffer>): void;
    on(type: "data", listener: (data: Iterable<number>) => undefined): void;
}

const PING = 0;
const WHOLE_CONFIG = 1;
const GET_PIN_POWER = 2;
const GET_PIN_MODE = 3;
const SET_PIN_POWER = 4;
const SET_PIN_MODE = 5;

export class BeadInterface extends EventEmitter<BeadInterfaceEventMap>
{
    #source: BeadInterfaceSource;
    #deserializer: Generator<undefined, IncomingPacket | typeof RESET, number>;
    #dataListener: (data: Iterable<number>) => undefined;
    #config: BeadConfig | null;
    #attemptSend: () => void;
    #attemptInterval: NodeJS.Timeout | null;
    #queue: any[] = [WHOLE_CONFIG, serialize({ type: "whole-config" })];

    constructor(options: BeadInterfaceOptions)
    {
        super();

        this.#deserializer = deserializePacket();
        this.#deserializer.next();

        this.#dataListener = (data) =>
        {
            for (const byte of data)
            {
                let done, value;

                try { ({ done, value } = this.#deserializer.next(byte)) }
                catch (error)
                {
                    this.#deserializer = deserializePacket();
                    this.#deserializer.next();

                    if (error instanceof SyntaxError)
                        this.emit("packet-error", error);
                }

                if (done)
                {
                    this.#deserializer = deserializePacket();
                    this.#deserializer.next();

                    if (value === RESET)
                        this.emit("reset");
                    else
                    {
                        const packet = value as IncomingPacket;

                        switch (packet.type)
                        {
                            case "pong":
                            {
                                break;
                            }
                            case "config":
                            {
                                this.#config =
                                {
                                    name: packet.name,
                                    pins: packet.pins.map((pin) => (
                                    {
                                        name: pin.name,
                                        supportsDigitalInput: pin.supportsDigitalInput,
                                        supportsDigitalOutput: pin.supportsDigitalOutput,
                                        supportsAnalogInput: pin.supportsAnalogInput,
                                        supportsAnalogOutput: pin.supportsAnalogOutput,
                                    })),
                                };

                                switch (this.#queue[0])
                                {
                                    case WHOLE_CONFIG:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(
                                                this.#queue[i],
                                                this,
                                                null,
                                                {
                                                    name: packet.name,
                                                    pins: packet.pins.map((pin) => (
                                                    {
                                                        name: pin.name,
                                                        supportsDigitalInput: pin.supportsDigitalInput,
                                                        supportsDigitalOutput: pin.supportsDigitalOutput,
                                                        supportsAnalogInput: pin.supportsAnalogInput,
                                                        supportsAnalogOutput: pin.supportsAnalogOutput,
                                                    })),
                                                });

                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                break;
                            }
                            case "get-pin-power-response":
                            {
                                switch (this.#queue[0])
                                {
                                    case GET_PIN_POWER:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(this.#queue[i],this, null, packet.power);
                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                break;
                            }
                            case "get-pin-mode-response":
                            {
                                switch (this.#queue[0])
                                {
                                    case GET_PIN_MODE:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(this.#queue[i],this, null, packet.mode);
                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                break;
                            }
                            case "set-pin-power-response":
                            {
                                switch (this.#queue[0])
                                {
                                    case SET_PIN_POWER:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(this.#queue[i],this, null);
                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                break;
                            }
                            case "set-pin-mode-response":
                            {
                                switch (this.#queue[0])
                                {
                                    case SET_PIN_MODE:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(this.#queue[i],this, null);
                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                break;
                            }
                            case "pin-listen":
                            {
                                this.emit(
                                    "pin-changed",
                                    packet.pin,
                                    (this.#config as BeadConfig).pins[packet.pin].name,
                                    packet.power);
                                break;
                            }
                            case "invalid-pin-mode":
                            {
                                console.warn("\x1B[33m/!\\\x1B[0m Received report of an invalid pin mode.");
                                break;
                            }
                            case "invalid-pin-id":
                            {
                                switch (this.#queue[0])
                                {
                                    case GET_PIN_POWER:
                                    case GET_PIN_MODE:
                                    case SET_PIN_POWER:
                                    case SET_PIN_MODE:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(this.#queue[i],this, "invalid-pin-id");
                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                console.warn("\x1B[33m/!\\\x1B[0m Received report of an invalid pin id.");
                                break;
                            }
                            case "invalid-packet-id":
                            {
                                console.warn("\x1B[33m/!\\\x1B[0m Received report of an invalid packet id.");
                                break;
                            }
                            case "invalid-write-to-input":
                            {
                                switch (this.#queue[0])
                                {
                                    case SET_PIN_POWER:
                                    case SET_PIN_MODE:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(this.#queue[i],this, "invalid-write-to-input");
                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                console.warn("\x1B[33m/!\\\x1B[0m Received report of an invalid write to input.");
                                break;
                            }
                            case "invalid-unsupported-mode":
                            {
                                switch (this.#queue[0])
                                {
                                    case SET_PIN_MODE:
                                    {
                                        let i = 2;
                                        while (typeof this.#queue[i] === "function")
                                        {
                                            Function.prototype.call.call(this.#queue[i],this, "invalid-unsupported-mode");
                                            i += 1;
                                        }

                                        this.#queue.splice(0, i);

                                        break;
                                    }
                                }

                                console.warn("\x1B[33m/!\\\x1B[0m Received report of the assignment of an unsupported pin mode.");
                                break;
                            }
                            case "invalid-escape":
                            {
                                console.warn("\x1B[33m/!\\\x1B[0m Received report of an invalid escape.");
                                break;
                            }
                        }

                        this.emit("packet", packet);

                        this.#attemptSend();

                        if (this.#queue.length === 0)
                        {
                            if (this.#attemptInterval !== null)
                            {
                                this.#attemptInterval.close();
                                this.#attemptInterval = null;
                            }
                        }
                        else
                        {
                            if (this.#attemptInterval === null)
                                this.#attemptInterval = setInterval(this.#attemptSend, 500);
                            else
                                this.#attemptInterval.refresh();
                        }
                    }
                }
            }
        };

        this.#source = options.source;
        this.#source.on("data", this.#dataListener);

        this.#config = null;

        this.#attemptSend = () => this.#source.write(this.#queue[1]);
        this.#attemptInterval = setInterval(this.#attemptSend, 500);
    }

    pinIdToName(
        id: number,
        callback: (this: BeadInterface, error: "invalid-pin-id" | null, name: string) => void)
    {
        if (this.#config === null)
        {
            let i = 2;
            while (typeof this.#queue[i] === "function") { i += 1 }

            this.#queue.splice(i, 0, (_: null, config: BeadConfig) =>
            {
                const pin = config.pins[id];
                if (pin === undefined)
                    Function.prototype.call.call(callback, "invalid-pin-id");
                else
                    Function.prototype.call.call(callback, null, pin.name);
            });
        }
        else
        {
            const pin = this.#config.pins[id];
            if (pin === undefined)
                Function.prototype.call.call(callback, "invalid-pin-id");
            else
                Function.prototype.call.call(callback, null, pin.name);
        }
    }

    pinNameToId(
        name: string,
        callback: (this: BeadInterface, error: "invalid-pin-id" | null, id: number) => void)
    {
        if (this.#config === null)
        {
            let i = 2;
            while (typeof this.#queue[i] === "function") { i += 1 }

            this.#queue.splice(i, 0, (_: null, config: BeadConfig) =>
            {
                for (let i = 0; i < config.pins.length; i += 1)
                {
                    if (config.pins[i].name === name)
                        return Function.prototype.call.call(callback, null, i);
                }

                Function.prototype.call.call(callback, "invalid-pin-id");
            });
        }
        else
        {
            for (let i = 0; i < this.#config.pins.length; i += 1)
            {
                if (this.#config.pins[i].name === name)
                    return Function.prototype.call.call(callback, null, i);
            }

            Function.prototype.call.call(callback, "invalid-pin-id");
        }
    }

    getPinPower(
        pin: number | string,
        callback: (this: BeadInterface, error: "invalid-pin-id" | null, power: number) => void): void
    {
        if (typeof pin === "string")
        {
            this.pinNameToId(pin, (error, pin) =>
            {
                if (error !== null)
                    return Function.prototype.call.call(callback, "invalid-pin-id");

                this.#queue.push(GET_PIN_POWER, serialize({ type: "get-pin-power", id: pin }), callback);
            });
        }
        else
        {
            if (this.#queue.length === 0)
            {
                if (this.#attemptInterval === null)
                    this.#attemptInterval = setInterval(this.#attemptSend, 500);
                else
                    this.#attemptInterval.refresh();

                this.#attemptSend();
            }

            this.#queue.push(GET_PIN_POWER, serialize({ type: "get-pin-power", id: pin }), callback);
        }
    }

    getPinMode(
        pin: number | string,
        callback: (this: BeadInterface, error: "invalid-pin-id" | null, mode: PinMode) => void): void
    {
        if (typeof pin === "string")
        {
            this.pinNameToId(pin, (error, pin) =>
            {
                if (error !== null)
                    return Function.prototype.call.call(callback, "invalid-pin-id");

                this.#queue.push(GET_PIN_MODE, serialize({ type: "get-pin-mode", id: pin }), callback);
            });
        }
        else
        {
            if (this.#queue.length === 0)
            {
                if (this.#attemptInterval === null)
                    this.#attemptInterval = setInterval(this.#attemptSend, 500);
                else
                    this.#attemptInterval.refresh();

                this.#attemptSend();
            }

            this.#queue.push(GET_PIN_MODE, serialize({ type: "get-pin-mode", id: pin }), callback);
        }
    }

    setPinPower(
        pin: number | string,
        power: number | boolean,
        callback: (this: BeadInterface, error: "invalid-pin-id" | null) => void): void
    {
        if (typeof pin === "string")
        {
            this.pinNameToId(pin, (error, pin) =>
            {
                if (error !== null)
                    return Function.prototype.call.call(callback, "invalid-pin-id");

                this.#queue.push(GET_PIN_POWER, serialize({ type: "set-pin-power", id: pin, power }), callback);
            });
        }
        else
        {
            if (this.#queue.length === 0)
            {
                if (this.#attemptInterval === null)
                    this.#attemptInterval = setInterval(this.#attemptSend, 500);
                else
                    this.#attemptInterval.refresh();

                this.#attemptSend();
            }

            this.#queue.push(GET_PIN_POWER, serialize({ type: "set-pin-power", id: pin, power }), callback);
        }
    }

    setPinMode(
        pin: number | string,
        mode: PinMode,
        callback: (this: BeadInterface, error: "invalid-pin-id" | null) => void): void
    {
        if (typeof pin === "string")
        {
            this.pinNameToId(pin, (error, pin) =>
            {
                if (error !== null)
                    return Function.prototype.call.call(callback, "invalid-pin-id");

                this.#queue.push(GET_PIN_MODE, serialize({ type: "set-pin-mode", id: pin, mode }), callback);
            });
        }
        else
        {
            if (this.#queue.length === 0)
            {
                if (this.#attemptInterval === null)
                    this.#attemptInterval = setInterval(this.#attemptSend, 500);
                else
                    this.#attemptInterval.refresh();

                this.#attemptSend();
            }

            this.#queue.push(GET_PIN_MODE, serialize({ type: "set-pin-mode", id: pin, mode }), callback);
        }
    }

    close()
    {
        this.#queue.length = 0;

        if (this.#attemptInterval !== null)
        {
            this.#attemptInterval.close();
            this.#attemptInterval = null;
        }
    }
}