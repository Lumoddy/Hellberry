import { Stream } from "node:stream";
import { PinMode, PinState } from "../pin.js";

const CONTROL_BYTE = "".charCodeAt(0);
const START_TEXT_BYTE = "".charCodeAt(0);

export interface OutgoingPacketMap
{
    "ping": {},
    "get-config": {},
    "get-pin": { id: number },
    "get-pin-mode": { id: number },
    "set-pin": { id: number, state: PinState },
    "set-pin-mode": { id: number, mode: PinMode },
}

export type OutgoingPacket = { [K in keyof OutgoingPacketMap]: { type: K } & OutgoingPacketMap[K] }[keyof OutgoingPacketMap];

export function encodePacket(packet: OutgoingPacket): Uint8Array
{
    const packetType = packet.type;
    switch (packetType)
    {
        case "ping":
        {
            return new Uint8Array([0x00]);
        }
        case "get-config":
        {
            return new Uint8Array([0x01]);
        }
        case "get-pin":
        {
            const id = packet.id;
            if (id === CONTROL_BYTE)
                return new Uint8Array([0x02, CONTROL_BYTE, CONTROL_BYTE]);
            else
                return new Uint8Array([0x02, id]);
        }
        case "get-pin-mode":
        {
            const id = packet.id;
            if (id === CONTROL_BYTE)
                return new Uint8Array([0x03, CONTROL_BYTE, CONTROL_BYTE]);
            else
                return new Uint8Array([0x03, id]);
        }
        case "set-pin":
        {
            let state;
            const packetState = packet.state;
            switch (packetState)
            {
                case "low": state = 0; break;
                case "high": state = 1; break;
                default:
                    throw new TypeError(`encodePacket: Invalid pin state "${packetState}".`);
            }

            const id = packet.id;
            if (id === CONTROL_BYTE)
                return new Uint8Array([0x02, CONTROL_BYTE, CONTROL_BYTE, state]);
            else
                return new Uint8Array([0x02, id, state]);
        }
        case "set-pin-mode":
        {
            let mode;
            const packetMode = packet.mode;
            switch (packetMode)
            {
                case "input": mode = 0; break;
                case "input-listening": mode = 1; break;
                case "output": mode = 2; break;
                default:
                    throw new TypeError(`encodePacket: Invalid pin mode "${packetMode}".`);
            }

            const id = packet.id;
            if (id === CONTROL_BYTE)
                return new Uint8Array([0x02, CONTROL_BYTE, CONTROL_BYTE, mode]);
            else
                return new Uint8Array([0x02, id, mode]);
        }
        default:
        {
            throw new TypeError(`encodePacket: Invalid packet type "${packetType}".`);
        }
    }
}