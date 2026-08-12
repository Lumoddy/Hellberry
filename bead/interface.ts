import { EventEmitter } from "node:events";
import { SerialPort } from "serialport";
import { deserializePacket, type IncomingPacket } from "./incoming.ts";
import type { DuplexEventMap } from "node:stream";
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
}

export class BeadInterface extends EventEmitter<BeadInterfaceEventMap>
{
    #serialPort: SerialPort;
    #deserializer: Generator<undefined, IncomingPacket | null, number>;
    #dataListener: (...args: DuplexEventMap["data"]) => void;
    #configPromise: Promise<BeadConfig>;
    #configResolve: ((arg?: any) => void) | null = null;
    #pingResolves: ((arg?: any) => void)[] = [];
    #getPinPowerResolves: ((arg?: any) => void)[] = [];
    #getPinModeResolves: ((arg?: any) => void)[] = [];
    #setPinPowerResolves: ((arg?: any) => void)[] = [];
    #setPinModeResolves: ((arg?: any) => void)[] = [];

    constructor(serialPort: SerialPort)
    {
        super();

        if (serialPort === null || typeof serialPort !== "object")
            throw new TypeError(
                `new BeadInterface: Argument 1 is not an object.`);
        else if (!(serialPort instanceof SerialPort))
            throw new TypeError(
                `new BeadInterface: Argument 1 does not implement interface SerialPort.`);

        this.#serialPort = serialPort;

        this.#deserializer = deserializePacket();
        this.#deserializer.next();

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

                    console.log("in", value.type === "config" ? "configured" : value);

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
    }

    async connect()
    {
        this.#serialPort.on("data", this.#dataListener);

        if (this.#configResolve !== null)
        {
            
            console.log("out", serialize("whole-config"));
            this.#serialPort.write(serialize("whole-config"));
            await this.#configPromise;
        }
    }

    disconnect()
    {
        this.#serialPort.off("data", this.#dataListener);
    }

    async config(): Promise<BeadConfig>
    {
        return { ...await this.#configPromise };
    }

    async idOfPin(pin: number | string): Promise<number>
    {
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
        else if (typeof pin === "number")
        {
            if (pin < 0 || pin >= config.pins.length)
                throw new RangeError(
                    `BeadInterface.idOfPin: Pin '${pin}' is out of range.`);
                
                return Math.trunc(pin);
        }
        else
            throw new TypeError(
                `BeadInterface.idOfPin: Argument 1 is not a number or string.`);
    }

    ping(): Promise<void>
    {
        return new Promise(async (resolve, reject) =>
        {
            
            console.log("out", serialize("ping"));
            this.#pingResolves.push(resolve, reject);
            this.#serialPort.write(serialize("ping"));
        });
    }

    getPinPower(pin: number | string): Promise<number>
    {
        return new Promise(async (resolve, reject) =>
        {
            
            console.log("out", serialize("get-pin-power", await this.idOfPin(pin)));
            this.#setPinPowerResolves.push(resolve, reject);
            this.#serialPort.write(serialize("get-pin-power", await this.idOfPin(pin)));
        });
    }

    getPinMode(pin: number | string): Promise<PinMode>
    {
        return new Promise(async (resolve, reject) =>
        {
            
            console.log("out", serialize("get-pin-mode", await this.idOfPin(pin)));
            this.#setPinModeResolves.push(resolve, reject);
            this.#serialPort.write(serialize("get-pin-mode", await this.idOfPin(pin)));
        });
    }

    setPinPower(pin: number | string, power: number | boolean): Promise<void>
    {
        return new Promise(async (resolve, reject) =>
        {
            
            console.log("out", serialize("set-pin-power", await this.idOfPin(pin), power));
            this.#setPinPowerResolves.push(resolve, reject);
            this.#serialPort.write(serialize("set-pin-power", await this.idOfPin(pin), power));
        });
    }

    setPinMode(pin: number | string, mode: PinMode): Promise<void>
    {
        return new Promise(async (resolve, reject) =>
        {
            
            console.log("out", serialize("set-pin-mode", await this.idOfPin(pin), mode));
            this.#setPinModeResolves.push(resolve, reject);
            this.#serialPort.write(serialize("set-pin-mode", await this.idOfPin(pin), mode));
        });
    }
}