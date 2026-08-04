import { Stream } from "node:stream";
import { PinMode, PinState } from "../pin.js";

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
            return new Uint8Array([0x02, packet.id]);
        }
        case "get-pin-mode":
        {
            return new Uint8Array([0x03, packet.id]);
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

            return new Uint8Array([0x04, packet.id, state]);
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

            return new Uint8Array([0x05, packet.id, mode]);
        }
        default:
        {
            throw new TypeError(`encodePacket: Invalid packet type "${packetType}".`);
        }
    }
}