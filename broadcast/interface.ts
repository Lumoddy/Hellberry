import * as os from "os";
import * as dgram from "dgram";
import * as net from "net";

export interface BroadcastInterfaceOptions
{
    readonly port: number,
}

export class BroadcastInterface
{
    #socket: dgram.Socket;
    #broadcastBuffer: any[] | null = [];

    constructor(options: BroadcastInterfaceOptions)
    {
        let localIp = undefined;
        let localIpMask = undefined;
        const networkInterfaces = os.networkInterfaces();
        search: for (const name in networkInterfaces)
        {
            const networkInterface = networkInterfaces[name] as os.NetworkInterfaceInfo[];
            for (const entry of networkInterface)
            {
                const address = entry.address;
                if (/^(?:192\.168|(?:10|172)\.\d+)\.\d+\.\d+$/.test(address))
                {
                    localIp = address;
                    localIpMask = entry.netmask;
                    break search;
                }
            }
        }

        if (localIp === undefined)
            throw new Error(
                `Failed to retrieve local IP address.`);

        const localIpMatch = /^\d+\.\d+\.\d+\.\d+$/.exec(localIp) as RegExpExecArray;
        const localIpMaskMatch = /^\d+\.\d+\.\d+\.\d+$/.exec(localIpMask as string) as RegExpExecArray;

        const broadcastIp = `${
            Number(localIpMatch[1]) | (~Number(localIpMaskMatch[1]) & 0xFF)}.${
            Number(localIpMatch[2]) | (~Number(localIpMaskMatch[2]) & 0xFF)}.${
            Number(localIpMatch[3]) | (~Number(localIpMaskMatch[3]) & 0xFF)}.${
            Number(localIpMatch[4]) | (~Number(localIpMaskMatch[4]) & 0xFF)}`;

        const socket = dgram.createSocket("udp4");
        this.#socket = socket;
        socket.bind(options.port, broadcastIp, () =>
        {
            socket.setBroadcast(true);

            const broadcastTriggerBuffer = this.#broadcastBuffer;
            if (broadcastTriggerBuffer !== null)
            {
                for (let i = 0; i < broadcastTriggerBuffer.length; i += 2)
                    this.#socket.send(broadcastTriggerBuffer[i + 0], broadcastTriggerBuffer[i + 1]);

                this.#broadcastBuffer = null;
            }
        });
    }

    address(): net.AddressInfo
    {
        if (!(this instanceof BroadcastInterface))
            throw new TypeError(
                `'address' called on an object that does not implement interface BroadcastInterface.`);

        return this.#socket.address();
    }

    broadcast(
        message: string | NodeJS.ArrayBufferView | readonly any[],
        callback?: (error: Error | null, bytes: number) => void)
    {
        if (!(this instanceof BroadcastInterface))
            throw new TypeError(
                `'broadcast' called on an object that does not implement interface BroadcastInterface.`);

        if (this.#broadcastBuffer === null)
            this.#socket.send(message, callback);
        else
            this.#broadcastBuffer.push(message, callback);
    }
}