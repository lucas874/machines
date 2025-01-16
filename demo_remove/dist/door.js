"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.s2 = exports.s1 = exports.s0 = void 0;
const factory_protocol_1 = require("./factory_protocol");
const machine_check_1 = require("@actyx/machine-check");
//import { protocol } from './protocol'
for (var p of factory_protocol_1.all_projections) {
    console.log(JSON.stringify(p));
}
//new Date().toLocaleString()
const door = factory_protocol_1.Composition.makeMachine('D');
exports.s0 = door.designEmpty('s0')
    .command('close', [factory_protocol_1.Events.time], () => { var dateString = new Date().toLocaleString(); console.log(dateString); return [factory_protocol_1.Events.time.make({ timeOfDay: dateString })]; })
    .finish();
exports.s1 = door.designEmpty('s1').finish();
exports.s2 = door.designEmpty('s2').finish();
exports.s0.react([factory_protocol_1.Events.partID], exports.s1, (_) => exports.s1.make());
exports.s1.react([factory_protocol_1.Events.part], exports.s0, (_) => exports.s0.make());
exports.s0.react([factory_protocol_1.Events.time], exports.s2, (_) => exports.s2.make());
const result_projection = (0, machine_check_1.projectCombineMachines)(factory_protocol_1.interfacing_swarms, factory_protocol_1.subs, "D");
if (result_projection.type == 'ERROR')
    throw new Error('error getting projection');
const projection = result_projection.data;
const cMap = new Map();
cMap.set(factory_protocol_1.Events.time.type, () => { var dateString = new Date().toLocaleString(); console.log(dateString); return [factory_protocol_1.Events.time.make({ timeOfDay: dateString })]; });
const rMap = new Map();
const statePayloadMap = new Map();
const fMap = { commands: cMap, reactions: rMap, statePayloads: statePayloadMap };
const mAnalysisResource = { initial: projection.initial, subscriptions: [], transitions: projection.transitions };
const [m3, i3] = factory_protocol_1.Composition.extendMachine("D", mAnalysisResource, factory_protocol_1.Events.allEvents, [door, exports.s0], fMap);
console.log(m3.createJSONForAnalysis(i3));
