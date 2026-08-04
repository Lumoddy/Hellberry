// import * as net from "node:net";
import * as sp from "serialport";

sp.SerialPort.list().then(console.log);

sp.ByteLengthParser