import type { PinMode } from "./interface.ts";

export interface OutgoingPacketArgMap
{
    "ping": { },
    "whole-config": { },
    "get-pin-power": { id: number },
    "get-pin-mode": { id: number },
    "set-pin-power": { id: number, power: number | boolean },
    "set-pin-mode": { id: number, mode: PinMode },
}

export type OutgoingPacket = ObjectFromMap<OutgoingPacketArgMap>;

const CONTROL_ESCAPE = 16;
const CONTROL_START = 2;

export function serialize(packet: OutgoingPacket): Buffer<ArrayBuffer>
{
    switch (packet.type)
    {
        case "ping": return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x00);
        case "whole-config": return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x01);
        case "get-pin-power":
        {
            const pin = packet.id & 0xFF;

            if (pin == CONTROL_ESCAPE)
                return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x02, pin, pin);
            else
                return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x02, pin);
        }
        case "get-pin-mode":
        {
            const pin = packet.id & 0xFF;

            if (pin == CONTROL_ESCAPE)
                return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x03, pin, pin);
            else
                return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x03, pin);
        }
        case "set-pin-power":
        {
            const pin = packet.id & 0xFF;

            const power = (Number(packet.power) * 0xFF) & 0xFF;

            if (pin == CONTROL_ESCAPE)
            {
                if (power == CONTROL_ESCAPE)
                    return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x04, pin, pin, power, power);
                else
                    return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x04, pin, pin, power);
            }
            else
            {
                if (power == CONTROL_ESCAPE)
                    return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x04, pin, power, power);
                else
                    return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x04, pin, power);
            }
        }
        case "set-pin-mode":
        {
            const pin = packet.id & 0xFF;

            let mode: number;
            switch (packet.mode)
            {
                case "digital-input": mode = 0; break;
                case "digital-listen": mode = 1; break;
                case "digital-output": mode = 2; break;
                case "analog-input": mode = 3; break;
                case "analog-output": mode = 4; break;
            }

            if (pin == CONTROL_ESCAPE)
                return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x05, pin, pin, mode);
            else
                return Buffer.of(CONTROL_ESCAPE, CONTROL_START, 0x05, pin, mode);
        }
    }
}