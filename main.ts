import * as process from "process";
import { runApp } from "./app.js";

const processArgs = process.argv;
setup: switch (processArgs[2])
{
    case "run":
    {
        let appPort: number | null = null;

        for (let i = 3; i < processArgs.length; i += 1)
        {
            const arg = processArgs[i];
            switch (arg)
            {
                case "-port":
                {
                    if (appPort !== null)
                    {
                        console.error(
                            `\x1B[31m{!}\x1B[0m Duplicate '-port' argument.`);
                        break setup;
                    }

                    i += 1;
                    if (i >= processArgs.length)
                    {
                        console.error(
                            `\x1B[31m{!}\x1B[0m Missing value after '-port' argument.`);
                        break setup;
                    }

                    const value = processArgs[i] as string;
                    appPort = Number(value);

                    if (!Number.isInteger(appPort))
                    {
                        console.error(
                            `\x1B[31m{!}\x1B[0m Invalid value for '-access' argument '${value}'.`);
                        break setup;
                    }

                    break;
                }
                default:
                {
                    console.error(
                        `\x1B[31m{!}\x1B[0m Unknown argument '${arg}'.`);
                    break setup;
                }
            }
        }

        if (appPort === null)
        {
            console.error(
                `\x1B[31m{!}\x1B[0m Missing '-port' argument.`);
            break setup;
        }

        runApp(
        {
            port: appPort,
        });

        break setup;
    }
    default:
    {
        console.log(`\x1b[1;32m->\x1b[0m The server part of the Hellberry View.
To use the view, write the shown pi address into any browser after executing the \x1b[1;36mrun\x1b[0m command.

Commands:

  \x1b[1;36mrun\x1b[0m Runs the server and listens for connections to display the dashboard on.
    \x1b[1;33m-port\x1b[0m Set the port to connect to, leave blank to let the OS decide.

  \x1b[1;36mhelp\x1b[0m Shows this screen.
`);

        break setup;
    }
}