import type { PinMode, BeadConfig, BeadPinConfig } from "./interface.ts";

export interface IncomingPacketObjectMap
{
    "pong": { },
    "config": BeadConfig,
    "get-pin-power-response": { power: number},
    "get-pin-mode-response": { mode: PinMode},
    "set-pin-power-response": { },
    "set-pin-mode-response": { },
    "pin-listen": { pin: number, power: number },
    "invalid-pin-mode": { },
    "invalid-pin-id": { },
    "invalid-packet-id": { },
    "invalid-write-to-input": { },
    "invalid-unsupported-mode": { },
    "invalid-escape": { },
}

export type IncomingPacket = ObjectFromMap<IncomingPacketObjectMap>;

const CONTROL_ESCAPE = 16;
const CONTROL_START = 2;

export const RESET = Symbol("reset");

export function* deserializePacket(): Generator<undefined, IncomingPacket | typeof RESET, number>
{
    const type = yield* deserializeUint8();
    if (type === RESET)
        return RESET;

    switch (type)
    {
        case 0:
        {
            return { type: "pong" };
        }
        case 1:
        {
            const name = yield* deserializeString();
            if (name === RESET)
                return RESET;
            const pins = yield* deserializeArray(deserializePinConfig);
            if (pins === RESET)
                return RESET;

            return { type: "config", name, pins };
        }
        case 2:
        {
            const power = yield* deserializeUint16();
            if (power === RESET)
                return RESET;

            return { type: "get-pin-power-response", power: power / 1023 };
        }
        case 3:
        {
            const mode = yield* deserializeUint8();
            if (mode === RESET)
                return RESET;

            switch (mode)
            {
                case 0: return { type: "get-pin-mode-response", mode: "digital-input" };
                case 1: return { type: "get-pin-mode-response", mode: "digital-listen" };
                case 2: return { type: "get-pin-mode-response", mode: "digital-output" };
                case 3: return { type: "get-pin-mode-response", mode: "analog-input" };
                case 4: return { type: "get-pin-mode-response", mode: "analog-output" };
                default:
                    throw new SyntaxError(
                        `deserialize: Invalid pin mode "${mode}".`);
            }
        }
        case 4:
        {
            return { type: "set-pin-power-response" };
        }
        case 5:
        {
            return { type: "set-pin-mode-response" };
        }
        case 6:
        {
            const pin = yield* deserializeUint8();
            if (pin === RESET)
                return RESET;
            const power = yield* deserializeUint16();
            if (power === RESET)
                return RESET;

            return { type: "pin-listen", pin, power: power / 1023 };
        }
        case 101:
        {
            return { type: "invalid-pin-mode" };
        }
        case 102:
        {
            return { type: "invalid-pin-id" };
        }
        case 103:
        {
            return { type: "invalid-packet-id" };
        }
        case 104:
        {
            return { type: "invalid-write-to-input" };
        }
        case 105:
        {
            return { type: "invalid-unsupported-mode" };
        }
        case 106:
        {
            return { type: "invalid-escape" };
        }
        default:
            throw new SyntaxError(
                `deserialize: Invalid packet type '${type}'.`);
    }
}

export function* deserializeUint8(): Generator<undefined, number | typeof RESET, number>
{
    const byte1 = Number(yield) & 0xFF;
    if (byte1 != CONTROL_ESCAPE)
        return byte1;

    const byte2 = Number(yield) & 0xFF;
    switch (byte2)
    {
        case CONTROL_ESCAPE: return byte2;
        case CONTROL_START: return RESET;
        default:
            throw new SyntaxError(
                `deserialize: Invalid escape sequence.`);
    }
}

export function* deserializeUint16(): Generator<undefined, number | typeof RESET, number>
{
    const byte0 = yield* deserializeUint8();
    if (byte0 === RESET)
        return RESET;
    const byte1 = yield* deserializeUint8();
    if (byte1 === RESET)
        return RESET;
    return byte0 | (byte1 << 8);
}

export function deserializeLength(): Generator<undefined, number | typeof RESET, number>
{
    return deserializeUint16();
}

export function* deserializeString(): Generator<undefined, string | typeof RESET, number>
{
    const buffer = yield* deserializeArray(deserializeUint8);
    if (buffer === RESET)
        return RESET;
    return String.fromCharCode(...buffer);
}

export function* deserializeArray<T>(
    deserialize: () => Generator<undefined, T | typeof RESET, number>): Generator<undefined, T[] | typeof RESET, number>
{
    const len = yield* deserializeLength();
    if (len === RESET)
        return RESET;

    const buffer = new Array(len);

    for (let i = 0; i < len; i += 1)
    {
        const item = yield* deserialize();
        if (item === RESET)
            return RESET;

        buffer[i] = item;
    }

    return buffer;
}

export function* deserializePinConfig(): Generator<undefined, BeadPinConfig | typeof RESET, number>
{
    const name = yield* deserializeString();
    if (name === RESET)
        return RESET;
    const flags = yield* deserializeUint8();
    if (flags === RESET)
        return RESET;

    return (
    {
        name,
        supportsDigitalInput: (flags & 1) != 0,
        supportsDigitalOutput: (flags & 2) != 0,
        supportsAnalogInput: (flags & 4) != 0,
        supportsAnalogOutput: (flags & 8) != 0,
    });
}