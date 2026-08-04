import { Transform, TransformCallback, TransformOptions } from "node:stream";
import { PinMode, PinState } from "../pin.js";

export interface IncomingPacketMap
{
    "ok": {},
    "whole-config":
    {
        name: string,
        pins: { id: number, name: string, readable: true, writeable: true }[],
    },
    "get-pin-response": { state: PinState },
    "get-pin-mode-response": { mode: PinMode },
    "pin-listen-change": { id: number, state: PinState },
    "bad-packet-error": {},
    "panic-error": {},
    "timeout-error": {},
    "invalid-pin-id-error": {},
    "invalid-pin-mode-error": {},
}

export type IncomingPacket = { [K in keyof IncomingPacketMap]: { type: K } & IncomingPacketMap[K] }[keyof IncomingPacketMap];

export interface IncomingPacketParserOptions extends TransformOptions
{
    littleEndian?: boolean | undefined | null,
}

const START = -1;

const CONTROL_BYTE = "".charCodeAt(0);
const START_TEXT_BYTE = "".charCodeAt(0);

function* unescapeByte(): Generator<undefined, number, number | typeof START>
{
    const byte = (yield) & 0xFF;

    switch (byte)
    {
        case CONTROL_BYTE:
        {
            switch (byte)
            {
                case CONTROL_BYTE: return CONTROL_BYTE;
                case START_TEXT_BYTE: return START;
                default:
                    throw new SyntaxError(
                        `IncomingPacketParser: Invalid escape sequence.`);
            }
        }
        default:
        {
            return byte;
        }
    }
}

function* parseUint8(_littleEndian: boolean = false): Generator<undefined, number, number>
{
    return (yield) & 0xFF;
}

function* parsePinState(_littleEndian: boolean = false): Generator<undefined, PinState, number>
{
    switch ((yield) & 0xFF)
    {
        case 0: return "low";
        case 1: return "high";
        default: throw new SyntaxError(
            `parsePinState: Invalid pin state value.`);
    }
}

function* parsePinMode(_littleEndian: boolean = false): Generator<undefined, PinMode, number>
{
    switch ((yield) & 0xFF)
    {
        case 0: return "input";
        case 1: return "input-listening";
        case 2: return "output";
        default: throw new SyntaxError(
            `parsePinMode: Invalid pin mode value.`);
    }
}

function* parseUint16(littleEndian: boolean): Generator<undefined, number, number>
{
    let byte0, byte1;
    if (littleEndian)
    {
        byte0 = (yield) & 0xFF;
        byte1 = (yield) & 0xFF;
    }
    else
    {
        byte1 = (yield) & 0xFF;
        byte0 = (yield) & 0xFF;
    }

    return (byte0 << 0) | (byte1 << 8);
}

function parseLength(littleEndian: boolean): Generator<undefined, number, number>
{
    return parseUint16(littleEndian);
}

function* parseString(littleEndian: boolean): Generator<undefined, string, number>
{
    let length = yield* parseLength(littleEndian);

    const buffer = new Uint8Array(length);
    for (let i = 0; i < length; i += 1)
    {
        buffer[i] = (yield) & 0xFF;
    }

    return String.fromCharCode(...buffer);
}

function* parsePacket(littleEndian: boolean): Generator<undefined, IncomingPacket, number>
{
    switch ((yield) & 0xFF)
    {
        case 0:
        {
            return { type: "ok" };
        }
        case 1:
        {
            let name = yield* parseString(littleEndian);

            const pinLength = yield* parseLength(littleEndian);
            const pins = new Array(pinLength);
            for (let i = 0; i < pinLength; i += 1)
            {
                const id = yield* parseUint8(littleEndian);
                const name = yield* parseString(littleEndian);
                const flags = yield* parseUint8(littleEndian);
                const readable = (flags & 0x1) !== 0;
                const writable = (flags & 0x2) !== 0;

                pins[i] = { id, name, readable, writable };
            }

            return { type: "whole-config", name, pins };
        }
        case 2:
        {
            const state = yield* parsePinState(littleEndian);

            return { type: "get-pin-response", state };
        }
        case 3:
        {
            const mode = yield* parsePinMode(littleEndian);

            return { type: "get-pin-mode-response", mode };
        }
        case 4:
        {
            const id = yield* parseUint8(littleEndian);
            const state = yield* parsePinState(littleEndian);

            return { type: "pin-listen-change", id, state };
        }
        case 100:
        {
            return { type: "bad-packet-error" };
        }
        case 101:
        {
            return { type: "panic-error" };
        }
        case 102:
        {
            return { type: "timeout-error" };
        }
        case 103:
        {
            return { type: "invalid-pin-id-error" };
        }
        case 104:
        {
            return { type: "invalid-pin-mode-error" };
        }
        default:
        {
            throw new SyntaxError(
                `parsePacket: Invalid packet type value.`);
        }
    }
}

export class IncomingPacketParser extends Transform
{
    #littleEndian: boolean;
    #escaper: Generator<unknown, number, number>;
    #parser: Generator<unknown, IncomingPacket, number>;

    constructor(options: IncomingPacketParserOptions)
    {
        super(options);

        this.#littleEndian = options.littleEndian ?? true;
        if (typeof this.#littleEndian !== "number")
            throw new TypeError(
                `new IncomingPacketParser: Option littleEndian is not a number.`);

        this.#escaper = unescapeByte();
        this.#escaper.next();
        this.#parser = parsePacket(this.#littleEndian);
        this.#parser.next();
    }

    override _transform(chunk: unknown, _encoding: BufferEncoding, callback: TransformCallback)
    {
        if (!(this instanceof IncomingPacketParser))
            throw new TypeError(
                `'_transform' called on an object that does not implement interface IncomingPacketParser.`);

        switch (true)
        {
            case chunk === null || typeof chunk !== "object":
            {
                throw new TypeError(
                    `IncomingPacketParser._transform: Argument 1 is not an object.`);
            }
            case chunk instanceof Uint8Array:
            {
                try
                {
                    const length = chunk.length;
                    for (let i = 0; i < length; i += 1)
                    {
                        const { done, value: byte } = this.#escaper.next(chunk[i]);
                        if (!done) continue;
                        this.#escaper = unescapeByte();
                        this.#escaper.next();
                        if (byte === START)
                        {
                            this.#parser = parsePacket(this.#littleEndian);
                            this.#parser.next();
                        }
                        else
                        {
                            const { done, value } = this.#parser.next(byte);
                            if (!done) continue;
                            this.#parser = parsePacket(this.#littleEndian);
                            this.#parser.next();
                            this.push(value);
                        }
                    }

                    callback();
                }
                catch (error)
                {
                    if (error instanceof SyntaxError)
                        callback(error);
                    else
                        throw error;
                }

                break;
            }
            default:
            {
                throw new TypeError(
                    `IncomingPacketParser._transform: Argument 1 does not implement interface Uint8Array.`);
            }
        }
    }
}