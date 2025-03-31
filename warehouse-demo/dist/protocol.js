"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.protocol = exports.Events = exports.manifest = void 0;
const machine_runner_1 = require("@actyx/machine-runner");
exports.manifest = {
    appId: 'com.example.tomato-robot',
    displayName: 'Tomato Robot',
    version: '1.0.0',
};
var Events;
(function (Events) {
    Events.HasWater = machine_runner_1.MachineEvent.design('HasWater').withoutPayload();
    Events.NeedsWater = machine_runner_1.MachineEvent.design('NeedsWater').withPayload();
    Events.Done = machine_runner_1.MachineEvent.design('Done').withoutPayload();
    Events.All = [Events.HasWater, Events.NeedsWater, Events.Done];
})(Events || (exports.Events = Events = {}));
exports.protocol = machine_runner_1.SwarmProtocol.make('wateringRobot', Events.All);
