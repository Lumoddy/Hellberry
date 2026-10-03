import { EventEmitter } from "events";
import { SerialPort } from "serialport";
import { deserializePacket, type IncomingPacket } from "./incoming.ts";
import type { DuplexEventMap } from "stream";
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
    "pin-changed": [pin: number, power: boolean],
    "pin-power-set": [pin: number, power: number | boolean],
    "pin-mode-set": [pin: number, mode: PinMode],
    "close": [];
    "end": [];
    "error": [err: Error];
}

export interface BeadInterfaceOptions
{
    readonly serialPort: SerialPort,
}

export class BeadInterface extends EventEmitter<BeadInterfaceEventMap>
{
    #serialPort: SerialPort;
    #deserializer: Generator<undefined, IncomingPacket | null, number>;
    #dataListener: (...args: DuplexEventMap["data"]) => void;
    #closeListener: (...args: DuplexEventMap["close"]) => void;
    #endListener: (...args: DuplexEventMap["end"]) => void;
    #errorListener: (...args: DuplexEventMap["error"]) => void;
    #configPromise: Promise<BeadConfig>;
    #configResolve: ((arg?: any) => void) | null;
    #pingResolves: ((arg?: any) => void)[] = [];
    #getPinPowerResolves: ((arg?: any) => void)[] = [];
    #getPinModeResolves: ((arg?: any) => void)[] = [];
    #setPinPowerResolves: ((arg?: any) => void)[] = [];
    #setPinModeResolves: ((arg?: any) => void)[] = [];

    get serialPort(): SerialPort { return this.#serialPort }

    constructor(options: BeadInterfaceOptions)
    {
        super();

        const serialPort = options.serialPort;

        if (serialPort === null || typeof serialPort !== "object")
            throw new TypeError(
                `new BeadInterface: Argument 1 is not an object.`);
        else if (!(serialPort instanceof SerialPort))
            throw new TypeError(
                `new BeadInterface: Argument 1 does not implement interface SerialPort.`);

        this.#serialPort = serialPort;

        this.#deserializer = deserializePacket();
        this.#deserializer.next();

        this.#configResolve = null;
        this.#configPromise = new Promise((resolve) => this.#configResolve = resolve);

        this.#dataListener = (data) =>
        {
            console.log(data);

            for (const byte of data)
            {
                const { done, value } = this.#deserializer.next(byte);
                if (done)
                {
                    this.#deserializer = deserializePacket();
                    this.#deserializer.next();

                    if (value === null)
                        continue;

                    switch (value.type)
                    {
                        case "pong":
                        {
                            for (let i = 0; i < this.#pingResolves.length; i += 2)
                                this.#pingResolves[i]();
                            this.#pingResolves.length = 0;
                            break;
                        }
                        case "config":
                        {
                            this.#configResolve?.(value);
                            this.#configResolve = null;
                            break;
                        }
                        case "get-pin-power-response":
                        {
                            for (let i = 0; i < this.#getPinPowerResolves.length; i += 2)
                                this.#getPinPowerResolves[i](value.power);
                            this.#getPinPowerResolves.length = 0;
                            break;
                        }
                        case "get-pin-mode-response":
                        {
                            for (let i = 0; i < this.#getPinModeResolves.length; i += 2)
                                this.#getPinModeResolves[i](value.mode);
                            this.#getPinModeResolves.length = 0;
                            break;
                        }
                        case "set-pin-power-response":
                        {
                            for (let i = 0; i < this.#setPinPowerResolves.length; i += 2)
                                this.#setPinPowerResolves[i]();
                            this.#setPinPowerResolves.length = 0;
                            break;
                        }
                        case "set-pin-mode-response":
                        {
                            for (let i = 0; i < this.#setPinModeResolves.length; i += 2)
                                this.#setPinModeResolves[i]();
                            this.#setPinModeResolves.length = 0;
                            break;
                        }
                        case "pin-listen":
                        {
                            this.emit("pin-changed", value.pin, value.power > 0);
                            break;
                        }
                        case "invalid-pin-mode":
                        case "invalid-pin-id":
                        case "invalid-packet-id":
                        case "invalid-write-to-input":
                        case "invalid-unsupported-mode":
                        case "invalid-invalid-escape":
                        {
                            if (this.#pingResolves.length === 0
                                && this.#getPinPowerResolves.length === 0
                                && this.#getPinModeResolves.length === 0
                                && this.#setPinPowerResolves.length === 0
                                && this.#setPinModeResolves.length === 0)
                            {
                                switch (value.type)
                                {
                                    case "invalid-pin-mode":
                                        throw new Error(`BeadInterface: Invalid pin mode.`);
                                    case "invalid-pin-id":
                                        throw new Error(`BeadInterface: Invalid pin id.`);
                                    case "invalid-packet-id":
                                        throw new Error(`BeadInterface: Invalid packet id.`);
                                    case "invalid-write-to-input":
                                        throw new Error(`BeadInterface: Invalid write to input.`);
                                    case "invalid-unsupported-mode":
                                        throw new Error(`BeadInterface: Invalid unsupported mode.`);
                                    case "invalid-invalid-escape":
                                        throw new Error(`BeadInterface: Invalid invalid escape.`);
                                }
                            }

                            for (let i = 1; i < this.#pingResolves.length; i += 2)
                                this.#pingResolves[i](value.type);
                            this.#pingResolves.length = 0;
                            for (let i = 1; i < this.#getPinPowerResolves.length; i += 2)
                                this.#getPinPowerResolves[i](value.type);
                            this.#getPinPowerResolves.length = 0;
                            for (let i = 1; i < this.#getPinModeResolves.length; i += 2)
                                this.#getPinModeResolves[i](value.type);
                            this.#getPinModeResolves.length = 0;
                            for (let i = 1; i < this.#setPinPowerResolves.length; i += 2)
                                this.#setPinPowerResolves[i](value.type);
                            this.#setPinPowerResolves.length = 0;
                            for (let i = 1; i < this.#setPinModeResolves.length; i += 2)
                                this.#setPinModeResolves[i](value.type);
                            this.#setPinModeResolves.length = 0;
                            break;
                        }
                    }
                }
            }
        };

        this.#closeListener = () => this.emit("close");
        serialPort.on("close", this.#closeListener);

        this.#endListener = () => this.emit("end");
        serialPort.on("end", this.#endListener);

        this.#errorListener = (err) => this.emit("error", err);
        serialPort.on("error", this.#errorListener);
    }

    async connect(): Promise<void>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'connect' called on an object that does not implement interface BeadInterface.`);

        this.#serialPort.on("data", this.#dataListener);

        if (this.#configResolve !== null)
        {
            this.#serialPort.write(serialize("whole-config"));
            await this.#configPromise;
        }
    }

    disconnect()
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'disconnect' called on an object that does not implement interface BeadInterface.`);

        this.#serialPort.off("data", this.#dataListener);
    }

    async config(): Promise<BeadConfig>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'config' called on an object that does not implement interface BeadInterface.`);

        return structuredClone(await this.#configPromise);
    }

    async idOfPin(pin: number | string): Promise<number>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'idOfPin' called on an object that does not implement interface BeadInterface.`);

        const config = await this.#configPromise;

        if (typeof pin === "string")
        {
            let found;
            const pins = config.pins;
            const length = pins.length;
            for (let i = 0; i < length; i += 1)
                if (pins[i].name === pin)
                    found = i;

            if (found === undefined)
                throw new RangeError(
                    `BeadInterface.idOfPin: Pin "${pin}" not found in config.`);

            return found;
        }
        else if (typeof pin === "number" && Number.isInteger(pin))
        {
            if (pin < 0 || pin >= config.pins.length)
                throw new RangeError(
                    `BeadInterface.idOfPin: Pin '${pin}' is out of range.`);

            return Math.trunc(pin);
        }
        else
            throw new TypeError(
                `BeadInterface.idOfPin: Argument 1 is not an integer or string.`);
    }

    async nameOfPin(pin: number | string): Promise<string>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'nameOfPin' called on an object that does not implement interface BeadInterface.`);

        const config = await this.#configPromise;

        if (typeof pin === "string")
        {
            for (const configPin of config.pins)
                if (configPin.name === pin)
                    return pin;

            throw new RangeError(
                `BeadInterface.nameOfPin: Pin '${pin}' not found in config.`);
        }
        else if (typeof pin === "number" && Number.isInteger(pin))
        {
            if (pin < 0 || pin >= config.pins.length)
                throw new RangeError(
                    `BeadInterface.idOfPin: Pin '${pin}' is out of range.`);

            return config.pins[pin].name;
        }
        else
            throw new TypeError(
                `BeadInterface.nameOfPin: Argument 1 is not an integer or string.`);
    }

    ping(): Promise<void>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'ping' called on an object that does not implement interface BeadInterface.`);

        return new Promise(async (resolve, reject) =>
        {
            this.#pingResolves.push(resolve, reject);
            await this.#configPromise;
            this.#serialPort.write(serialize("ping"));
        });
    }

    getPinPower(pin: number | string): Promise<number>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'getPinPower' called on an object that does not implement interface BeadInterface.`);

        return new Promise(async (resolve, reject) =>
        {
            this.#setPinPowerResolves.push(resolve, reject);
            this.#serialPort.write(serialize("get-pin-power", await this.idOfPin(pin)));
        });
    }

    getPinMode(pin: number | string): Promise<PinMode>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'getPinMode' called on an object that does not implement interface BeadInterface.`);

        return new Promise(async (resolve, reject) =>
        {
            this.#setPinModeResolves.push(resolve, reject);
            this.#serialPort.write(serialize("get-pin-mode", await this.idOfPin(pin)));
        });
    }

    setPinPower(pin: number | string, power: number | boolean): Promise<void>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'setPinPower' called on an object that does not implement interface BeadInterface.`);

        return new Promise(async (resolve, reject) =>
        {
            this.#setPinPowerResolves.push(resolve, reject);
            const pinId = await this.idOfPin(pin);
            this.#serialPort.write(serialize("set-pin-power", pinId, power));
            this.emit("pin-power-set", pinId, power);
        });
    }

    setPinMode(pin: number | string, mode: PinMode): Promise<void>
    {
        if (!(this instanceof BeadInterface))
            throw new TypeError(
                `'setPinMode' called on an object that does not implement interface BeadInterface.`);

        return new Promise(async (resolve, reject) =>
        {
            this.#setPinModeResolves.push(resolve, reject);
            const pinId = await this.idOfPin(pin);
            this.#serialPort.write(serialize("set-pin-mode", pinId, mode));
            this.emit("pin-mode-set", pinId, mode);
        });
    }
}