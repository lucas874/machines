"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.all_projections = exports.subs = exports.interfacing_swarms = exports.Gquality = exports.Gfactory = exports.Gwarehouse = exports.Composition = exports.Events = exports.manifest = void 0;
exports.getRandomInt = getRandomInt;
/* eslint-disable @typescript-eslint/no-namespace */
const machine_runner_1 = require("@actyx/machine-runner");
const machine_check_1 = require("@actyx/machine-check");
exports.manifest = {
    appId: 'com.example.car-factory',
    displayName: 'Car Factory',
    version: '1.0.0',
};
var Events;
(function (Events) {
    Events.partReq = machine_runner_1.MachineEvent.design('partReq').withPayload();
    Events.partOK = machine_runner_1.MachineEvent.design('partOK').withPayload();
    Events.pos = machine_runner_1.MachineEvent.design('pos').withPayload();
    Events.closingTime = machine_runner_1.MachineEvent.design('closingTime').withPayload();
    Events.car = machine_runner_1.MachineEvent.design('car').withPayload();
    Events.observing = machine_runner_1.MachineEvent.design('obs').withoutPayload();
    Events.report = machine_runner_1.MachineEvent.design('report').withPayload();
    Events.allEvents = [Events.partReq, Events.partOK, Events.pos, Events.closingTime, Events.car, Events.observing, Events.report];
})(Events || (exports.Events = Events = {}));
exports.Composition = machine_runner_1.SwarmProtocol.make('Composition', Events.allEvents);
exports.Gwarehouse = {
    initial: '0',
    transitions: [
        { source: '0', target: '1', label: { cmd: 'request', role: 'T', logType: [Events.partReq.type] } },
        { source: '1', target: '2', label: { cmd: 'get', role: 'FL', logType: [Events.pos.type] } },
        { source: '2', target: '0', label: { cmd: 'deliver', role: 'T', logType: [Events.partOK.type] } },
        { source: '0', target: '3', label: { cmd: 'close', role: 'D', logType: [Events.closingTime.type] } },
    ]
};
exports.Gfactory = {
    initial: '0',
    transitions: [
        { source: '0', target: '1', label: { cmd: 'request', role: 'T', logType: [Events.partReq.type] } },
        { source: '1', target: '2', label: { cmd: 'deliver', role: 'T', logType: [Events.partOK.type] } },
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
exports.interfacing_swarms = [{ protocol: exports.Gwarehouse, interface: null }, { protocol: exports.Gfactory, interface: 'T' }, { protocol: exports.Gquality, interface: 'R' }];
const result_subs = (0, machine_check_1.overapproxWWFSubscriptions)(exports.interfacing_swarms, {}, 'TwoStep');
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
