"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
var __asyncValues = (this && this.__asyncValues) || function (o) {
    if (!Symbol.asyncIterator) throw new TypeError("Symbol.asyncIterator is not defined.");
    var m = o[Symbol.asyncIterator], i;
    return m ? m.call(o) : (o = typeof __values === "function" ? __values(o) : o[Symbol.iterator](), i = {}, verb("next"), verb("throw"), verb("return"), i[Symbol.asyncIterator] = function () { return this; }, i);
    function verb(n) { i[n] = o[n] && function (v) { return new Promise(function (resolve, reject) { v = o[n](v), settle(resolve, reject, v.done, v.value); }); }; }
    function settle(resolve, reject, d, v) { Promise.resolve(v).then(function(v) { resolve({ value: v, done: d }); }, reject); }
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.s2 = exports.s1 = exports.s0 = exports.sub = void 0;
const sdk_1 = require("@actyx/sdk");
const machine_runner_1 = require("@actyx/machine-runner");
const factory_protocol_1 = require("./factory_protocol");
const machine_check_1 = require("@actyx/machine-check");
// Generate a subscription w.r.t. which Gwarehouse || Gfactory || Gquality is well-formed
const result_sub = (0, machine_check_1.overapproxWWFSubscriptions)(factory_protocol_1.interfacing_swarms, {}, 'Medium');
if (result_sub.type === 'ERROR')
    throw new Error(result_sub.errors.join(', '));
exports.sub = result_sub.data;
// Check well-formedness (only here for demonstration purposes)
const checkResult = (0, machine_check_1.checkWWFSwarmProtocol)(factory_protocol_1.interfacing_swarms, exports.sub);
if (checkResult.type == 'ERROR')
    throw new Error(checkResult.errors.join(", "));
// Using the machine runner DSL an implmentation of robot in Gfactory is:
const robot = factory_protocol_1.Composition.makeMachine('R');
exports.s0 = robot.designEmpty('s0').finish();
exports.s1 = robot.designState('s1').withPayload()
    .command("build", [factory_protocol_1.Events.car], (s, _) => {
    var modelName = s.self.part === 'spoiler' ? "sports car" : "sedan";
    console.log("using the ", s.self.part, " to build a ", modelName);
    return [factory_protocol_1.Events.car.make({ part: s.self.part, modelName: modelName })];
})
    .finish();
exports.s2 = robot.designEmpty('s2').finish();
exports.s0.react([factory_protocol_1.Events.part], exports.s1, (_, e) => {
    console.log("received a ", e.payload.part);
    return exports.s1.make({ part: e.payload.part });
});
exports.s1.react([factory_protocol_1.Events.car], exports.s2, (_) => exports.s2.make());
// Projection of Gwarehouse || Gfactory || Gquality over R
//const result_projection = projectCombineMachines(interfacing_swarms, sub, "R")
//if (result_projection.type == 'ERROR') throw new Error('error getting projection')
//const projection = result_projection.data
const result_projection_info = (0, machine_check_1.projectionAndInformation)(factory_protocol_1.interfacing_swarms, exports.sub, "R");
if (result_projection_info.type == 'ERROR')
    throw new Error('error getting projection');
const projection_info = result_projection_info.data;
console.log(projection_info);
// Command map
const cMap = new Map();
cMap.set(factory_protocol_1.Events.car.type, (s, _) => {
    var modelName = s.self.part === "spoiler" ? "sports car" : "sedan";
    console.log("using the ", s.self.part, " to build a ", modelName);
    return { part: s.self.part, modelName: modelName };
});
//return [Events.car.make({part: s.self.part, modelName: modelName})]})
// Reaction map
const rMap = new Map();
const partReaction = {
    genPayloadFun: (_, e) => {
        console.log("received a ", e.payload.part);
        return { part: e.payload.part };
    }
};
rMap.set(factory_protocol_1.Events.part.type, partReaction);
const fMap = { commands: cMap, reactions: rMap, initialPayloadType: undefined };
// Extend machine
const [m3, i3] = factory_protocol_1.Composition.extendMachineBT("R", projection_info, factory_protocol_1.Events.allEvents, fMap, exports.s0);
// Check machine (for demonstration purposes)
const checkProjResult = (0, machine_check_1.checkComposedProjection)(factory_protocol_1.interfacing_swarms, exports.sub, "R", m3.createJSONForAnalysis(i3));
if (checkProjResult.type == 'ERROR')
    throw new Error(checkProjResult.errors.join(", "));
// Run the extended machine
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const app = yield sdk_1.Actyx.of(factory_protocol_1.manifest);
        const tags = factory_protocol_1.Composition.tagWithEntityId('factory-1');
        const machine = (0, machine_runner_1.createMachineRunnerBT)(app, tags, i3, undefined);
        try {
            for (var _d = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _d = true) {
                _c = machine_1_1.value;
                _d = false;
                const state = _c;
                console.log("robot. state is:", state.type);
                if (state.payload !== undefined) {
                    console.log("state payload is:", state.payload);
                }
                console.log();
                const s = state.cast();
                for (var c in s.commands()) {
                    if (c === 'build') {
                        setTimeout(() => {
                            var _a, _b;
                            var s1 = (_b = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.cast()) === null || _b === void 0 ? void 0 : _b.commands();
                            if (Object.keys(s1 || {}).includes('build')) {
                                s1.build();
                            }
                        }, (0, factory_protocol_1.getRandomInt)(4000, 8000));
                        break;
                    }
                }
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (!_d && !_a && (_b = machine_1.return)) yield _b.call(machine_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
        app.dispose();
    });
}
main();
