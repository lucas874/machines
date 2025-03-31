"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.all_projections = exports.subs = exports.interfacing_swarms = exports.Gwarehouse = exports.Composition = exports.Events = exports.manifest = void 0;
exports.getRandomInt = getRandomInt;
exports.loopThroughJSON = loopThroughJSON;
/* eslint-disable @typescript-eslint/no-namespace */
const machine_runner_1 = require("@actyx/machine-runner");
const machine_check_1 = require("@actyx/machine-check");
exports.manifest = {
    appId: 'com.example.car-factory',
    displayName: 'Car Factory',
    version: '1.0.0',
};
/*
 * Example from CoPLaWS slides by Florian Furbach
 */
var Events;
(function (Events) {
    Events.partID = machine_runner_1.MachineEvent.design('partID').withPayload();
    Events.part = machine_runner_1.MachineEvent.design('part').withPayload();
    Events.position = machine_runner_1.MachineEvent.design('position').withPayload();
    Events.time = machine_runner_1.MachineEvent.design('time').withPayload();
    Events.allEvents = [Events.partID, Events.part, Events.position, Events.time];
})(Events || (exports.Events = Events = {}));
exports.Composition = machine_runner_1.SwarmProtocol.make('Composition', Events.allEvents);
exports.Gwarehouse = {
    initial: '0',
    transitions: [
        { source: '0', target: '1', label: { cmd: 'request', role: 'T', logType: [Events.partID.type] } },
        { source: '1', target: '2', label: { cmd: 'get', role: 'FL', logType: [Events.position.type] } },
        { source: '2', target: '0', label: { cmd: 'deliver', role: 'T', logType: [Events.part.type] } },
        { source: '0', target: '3', label: { cmd: 'close', role: 'D', logType: [Events.time.type] } },
    ]
};
exports.interfacing_swarms = [{ protocol: exports.Gwarehouse, interface: null }];
const result_subs = (0, machine_check_1.overapproxWWFSubscriptions)(exports.interfacing_swarms, {}, 'Medium');
if (result_subs.type === 'ERROR')
    throw new Error(result_subs.errors.join(', '));
exports.subs = result_subs.data;
const result_project_all = (0, machine_check_1.projectAll)(exports.interfacing_swarms, exports.subs);
if (result_project_all.type === 'ERROR')
    throw new Error('error getting subscription');
exports.all_projections = result_project_all.data;
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Math/random
function getRandomInt(min, max) {
    const minCeiled = Math.ceil(min);
    const maxFloored = Math.floor(max);
    return Math.floor(Math.random() * (maxFloored - minCeiled) + minCeiled); // The maximum is exclusive and the minimum is inclusive
}
// https://medium.com/@alaneicker/how-to-process-json-data-with-recursion-dc530dd3db09
function loopThroughJSON(k, obj) {
    for (let key in obj) {
        if (typeof obj[key] === 'object') {
            if (Array.isArray(obj[key])) {
                // loop through array
                for (let i = 0; i < obj[key].length; i++) {
                    loopThroughJSON(k + " " + key, obj[key][i]);
                }
            }
            else {
                // call function recursively for object
                loopThroughJSON(k + " " + key, obj[key]);
            }
        }
        else {
            // do something with value
            console.log(k + " " + key + ': ', obj[key]);
        }
    }
}
