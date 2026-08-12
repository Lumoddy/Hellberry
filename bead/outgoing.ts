import type { PinMode } from "./interface.ts";

export interface OutgoingPacketArgMap
{
    "ping": [],
    "whole-config": [],
    "get-pin-power": [id: number],
    "get-pin-mode": [id: number],
    "set-pin-power": [id: number, power: number | boolean],
    "set-pin-mode": [id: number, mode: PinMode],
}

const CONTROL_ESCAPE = 16;
const CONTROL_START = 2;

export function serialize<T extends keyof OutgoingPacketArgMap>(
    type: T,
    ...args: OutgoingPacketArgMap[T]): number[]
{
    switch (type)
    {
        case "ping": return [CONTROL_ESCAPE, CONTROL_START, 0x00];
        case "whole-config": return [CONTROL_ESCAPE, CONTROL_START, 0x01];
        case "get-pin-power":
        {
            const pin = Number(args[0]) & 0xFF;
            if (typeof pin !== "number")
                throw new TypeError(
                    `serialize: Argument 1 is not a number.`);

            if (pin == CONTROL_ESCAPE)
                return [CONTROL_ESCAPE, CONTROL_START, 0x02, CONTROL_ESCAPE, CONTROL_ESCAPE];
            else
                return [CONTROL_ESCAPE, CONTROL_START, 0x02, pin];
        }
        case "get-pin-mode":
        {
            const pin = Number(args[0]) & 0xFF;
            if (typeof pin !== "number")
                throw new TypeError(
                    `serialize: Argument 1 is not a number.`);

            if (pin == CONTROL_ESCAPE)
                return [CONTROL_ESCAPE, CONTROL_START, 0x03, CONTROL_ESCAPE, CONTROL_ESCAPE];
            else
                return [CONTROL_ESCAPE, CONTROL_START, 0x03, pin];
        }
        case "set-pin-power":
        {
            const pin = Number(args[0]) & 0xFF;
            if (typeof pin !== "number")
                throw new TypeError(
                    `serialize: Argument 1 is not a number.`);

            const power = (Number(args[1]) * 0xFF) & 0xFF;
            if (typeof power !== "number")
                throw new TypeError(
                    `serialize: Argument 2 is not a number.`);

            if (pin == CONTROL_ESCAPE)
            {
                if (power == CONTROL_ESCAPE)
                    return [CONTROL_ESCAPE, CONTROL_START, 0x04, CONTROL_ESCAPE, CONTROL_ESCAPE, CONTROL_ESCAPE, CONTROL_ESCAPE];
                else
                    return [CONTROL_ESCAPE, CONTROL_START, 0x04, CONTROL_ESCAPE, CONTROL_ESCAPE, power];
            }
            else
            {
                if (power == CONTROL_ESCAPE)
                    return [CONTROL_ESCAPE, CONTROL_START, 0x04, pin, CONTROL_ESCAPE, CONTROL_ESCAPE];
                else
                    return [CONTROL_ESCAPE, CONTROL_START, 0x04, pin, power];
            }
        }
        case "set-pin-mode":
        {
            const pin = Number(args[0]) & 0xFF;
            if (typeof pin !== "number")
                throw new TypeError(
                    `serialize: Argument 1 is not a number.`);

            let mode: number;
            switch (args[1])
            {
                case "digital-input": mode = 0; break;
                case "digital-listen": mode = 1; break;
                case "digital-output": mode = 2; break;
                case "analog-input": mode = 3; break;
                case "analog-output": mode = 4; break;
                default:
                    throw new TypeError(
                        `serialize: Argument 2 is not a valid pin mode.`);
            }

            if (pin == CONTROL_ESCAPE)
                return [CONTROL_ESCAPE, CONTROL_START, 0x05, CONTROL_ESCAPE, CONTROL_ESCAPE, mode];
            else
                return [CONTROL_ESCAPE, CONTROL_START, 0x05, pin, mode];
        }
        default:
            throw new TypeError(
                `serialize: Argument 1 is not a valid packet type.`);
    }
}