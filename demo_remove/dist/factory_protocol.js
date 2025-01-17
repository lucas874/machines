"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.all_projections = exports.subsf = exports.subswh = exports.subs = exports.interfacing_swarmsf = exports.interfacing_swarmswh = exports.interfacing_swarms = exports.Gquality = exports.Gfactory = exports.Gwarehouse = exports.Composition = exports.Events = exports.manifest = void 0;
exports.getRandomInt = getRandomInt;
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
    Events.part = machine_runner_1.MachineEvent.design('part').withoutPayload();
    Events.position = machine_runner_1.MachineEvent.design('position').withPayload();
    Events.time = machine_runner_1.MachineEvent.design('time').withPayload();
    Events.car = machine_runner_1.MachineEvent.design('car').withoutPayload();
    Events.observing = machine_runner_1.MachineEvent.design('ok').withoutPayload();
    Events.report = machine_runner_1.MachineEvent.design('report').withoutPayload();
    Events.allEvents = [Events.partID, Events.part, Events.position, Events.time, Events.car, Events.observing, Events.report];
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
exports.Gfactory = {
    initial: '0',
    transitions: [
        { source: '0', target: '1', label: { cmd: 'request', role: 'T', logType: [Events.partID.type] } },
        { source: '1', target: '2', label: { cmd: 'deliver', role: 'T', logType: [Events.part.type] } },
        { source: '2', target: '3', label: { cmd: 'build', role: 'R', logType: [Events.car.type] } },
    ]
};
exports.Gquality = {
    initial: '0',
    transitions: [
        { source: '0', target: '1', label: { cmd: 'observe', role: 'QCR', logType: [Events.observing.type] } },
        { source: '1', target: '2', label: { cmd: 'build', role: 'R', logType: [Events.car.type] } },
        { source: '2', target: '3', label: { cmd: 'test', role: 'QCR', logType: [Events.report.type] } },
    ]
};
//const protocols: InterfacingSwarms = [{protocol: Gwarehouse, interface: null}, {protocol: Gfactory, interface: 'T'}, {protocol: Gquality, interface: 'R'}]
exports.interfacing_swarms = [{ protocol: exports.Gwarehouse, interface: null }, { protocol: exports.Gfactory, interface: 'T' }];
exports.interfacing_swarmswh = [{ protocol: exports.Gwarehouse, interface: null }];
exports.interfacing_swarmsf = [{ protocol: exports.Gfactory, interface: null }];
const result_subs = (0, machine_check_1.overapproxWWFSubscriptions)(exports.interfacing_swarms, {}, 'Medium');
if (result_subs.type === 'ERROR')
    throw new Error(result_subs.errors.join(', '));
exports.subs = result_subs.data;
const result_subswh = (0, machine_check_1.overapproxWWFSubscriptions)(exports.interfacing_swarmswh, {}, 'Medium');
if (result_subswh.type === 'ERROR')
    throw new Error(result_subswh.errors.join(', '));
exports.subswh = result_subswh.data;
const result_subsf = (0, machine_check_1.overapproxWWFSubscriptions)(exports.interfacing_swarmsf, {}, 'Medium');
if (result_subsf.type === 'ERROR')
    throw new Error(result_subsf.errors.join(', '));
exports.subsf = result_subsf.data;
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
