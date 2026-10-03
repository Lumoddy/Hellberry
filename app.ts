import * as http from "http";
import * as path from "path";
import * as ws from "ws";
import * as os from "os";
import { BeadInterface } from "./bead/interface.ts";
import { SerialHandler } from "./serial_handler/interface.ts";

type Config =
{
    port: number,
};

export function runApp(config: Config)
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
                setTimeout(() => handler.connect(), 2000);

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