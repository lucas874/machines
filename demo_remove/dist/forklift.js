"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.s2 = exports.s1 = exports.s0 = void 0;
const factory_protocol_1 = require("./factory_protocol");
const machine_check_1 = require("@actyx/machine-check");
/* type partIDToGet = {
    pid: string
} */
const forklift = factory_protocol_1.Composition.makeMachine('FL');
exports.s0 = forklift.designEmpty('s0').finish();
exports.s1 = forklift.designEmpty('s1')
    .command('get', [factory_protocol_1.Events.position], () => [{}])
    .finish();
exports.s2 = forklift.designEmpty('s2').finish();
/* console.log("sub comp: ", JSON.stringify(subs))
console.log("sub wh: ", JSON.stringify(subswh))
console.log("sub f: ", JSON.stringify(subsf)) */
exports.s0.react([factory_protocol_1.Events.partID], exports.s1, (_) => exports.s1.make());
exports.s1.react([factory_protocol_1.Events.position], exports.s0, (_) => exports.s0.make());
exports.s0.react([factory_protocol_1.Events.time], exports.s2, (_) => exports.s2.make());
const result_projection = (0, machine_check_1.projectCombineMachines)(factory_protocol_1.interfacing_swarms, factory_protocol_1.subs, "FL");
if (result_projection.type == 'ERROR')
    throw new Error('error getting projection');
const projection = result_projection.data;
const cMap = new Map();
const rMap = new Map();
const statePayloadMap = new Map();
const fMap = { commands: cMap, reactions: rMap, statePayloads: statePayloadMap };
const mAnalysisResource = { initial: projection.initial, subscriptions: [], transitions: projection.transitions };
const [m3, i3] = factory_protocol_1.Composition.extendMachine("D", mAnalysisResource, factory_protocol_1.Events.allEvents, [forklift, exports.s0], fMap);
console.log(m3.createJSONForAnalysis(i3));
