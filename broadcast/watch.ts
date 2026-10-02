import * as dgram from "dgram";

export interface BroadcastWatchOptions
{
    readonly port: number,
}

export function broadcastWatch(options: BroadcastWatchOptions, callback?: () => void): dgram.Socket
{
    const socket = dgram.createSocket("udp4");
    socket.bind(options.port, () =>
    {
        socket.setBroadcast(true);
        callback?.();
    });

    return socket;
}