"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.s2 = exports.s1 = exports.s0 = void 0;
const factory_protocol_1 = require("./factory_protocol");
const machine_check_1 = require("@actyx/machine-check");
const robot = factory_protocol_1.Composition.makeMachine('R');
exports.s0 = robot.designEmpty('s0').finish();
exports.s1 = robot.designEmpty('s1')
    .command("build", [factory_protocol_1.Events.car], () => [{}])
    .finish();
exports.s2 = robot.designEmpty('s2').finish();
exports.s0.react([factory_protocol_1.Events.part], exports.s1, (_) => exports.s1.make());
exports.s1.react([factory_protocol_1.Events.car], exports.s2, (_) => exports.s2.make());
const result_projection = (0, machine_check_1.projectCombineMachines)(factory_protocol_1.interfacing_swarms, factory_protocol_1.subs, "R");
if (result_projection.type == 'ERROR')
    throw new Error('error getting projection');
const projection = result_projection.data;
const cMap = new Map();
const rMap = new Map();
const statePayloadMap = new Map();
const fMap = { commands: cMap, reactions: rMap, statePayloads: statePayloadMap };
const mAnalysisResource = { initial: projection.initial, subscriptions: [], transitions: projection.transitions };
const [m3, i3] = factory_protocol_1.Composition.extendMachine("R", mAnalysisResource, factory_protocol_1.Events.allEvents, [robot, exports.s0], fMap);
console.log(m3.createJSONForAnalysis(i3));
