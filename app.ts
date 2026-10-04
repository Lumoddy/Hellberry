import * as http from "http";
import * as path from "path";
import * as ws from "ws";
import * as os from "os";
import { BeadInterface, type PinMode } from "./bead/interface.ts";
import { SerialHandler } from "./serial_handler/interface.ts";

export type AppConfig =
{
    port: number,
};

export function runApp(config: AppConfig)
{
    const appPort = config.port;

    const server = http.createServer((request, response) =>
    {
        response.statusCode = 404;
        response.end();
    });

    const wss = new ws.WebSocketServer({ noServer: true });

    wss.on("connection", (ws, request) =>
    {
        interface IncomingPacketMap
        {
            "ping": { },
            "whole-config": { },
            "get-pin-power": { device: string, pin: string },
            "get-pin-mode": { device: string, pin: string },
            "set-pin-power": { device: string, pin: string, power: number | boolean },
            "set-pin-mode": { device: string, pin: string, mode: PinMode },
        }

        interface OutgoingPacketMap
        {
            "pong": { },
            "config":
            {
                devices:
                {
                    name: string,
                    pins:
                    {
                        name: string,
                        supportsDigitalInput: boolean,
                        supportsDigitalOutput: boolean,
                        supportsAnalogInput: boolean,
                        supportsAnalogOutput: boolean,
                    }[],
                }[],
            },
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

        type OutgoingPacket = ObjectFromMap<OutgoingPacketMap>;

        ws.on("message", (message) =>
        {
            
        });

        ws.on("close", () =>
        {
            
        });
    });

    server.on("upgrade", (request, socket, head) =>
    {
        const url = request.url;

        if (url !== undefined && path.isAbsolute(url))
        {
            if (/^ws$/.test(url) && request.method === "GET")
            {
                wss.handleUpgrade(request, socket, head, (ws) =>
                {
                    wss.emit("connection", ws, request);
                });

                return;
            }
        }
    });

    let localIp: string | null = null;

    const networkInterfaces = os.networkInterfaces();
    for (const key in networkInterfaces)
    {
        for (const networkInterface of networkInterfaces[key] as os.NetworkInterfaceInfo[])
        {
            if (networkInterface.family === "IPv4")
            {
                localIp = networkInterface.address;
            }
        }
    }

    if (localIp === null)
    {
        console.error(`\x1b[1;31m{!}\x1b[0m Failed to retrieve local IP address.`);
    }
    else
    {
        server.listen(appPort ?? 0, localIp);

        const serialHandler = new SerialHandler(
        {
            condition: (info) => info.manufacturer !== undefined && /[Aa]rduino/.test(info.manufacturer),
            construct: BeadInterface,
            created(handler)
            {
                handler.on("pin-changed", (pin, power) =>
                {
                    for (const ws of wss.clients)
                    {
                        ws.send(JSON.stringify(
                        {
                            type: "pin-changed",
                            pin: pin,
                            name: handler.nameOfPin(pin),
                            power: power,
                        }));
                    }
                });

                handler.on("close", () =>
                {

                });
            },
        });

        setInterval(() => serialHandler.update(), 1000);
    }
}