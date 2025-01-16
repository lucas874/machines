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
exports.s2 = exports.s1 = exports.s0 = void 0;
const sdk_1 = require("@actyx/sdk");
const machine_runner_1 = require("@actyx/machine-runner");
const factory_protocol_1 = require("./factory_protocol");
const machine_check_1 = require("@actyx/machine-check");
/* for (var p of all_projections) {
    console.log(JSON.stringify(p))
} */
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
//console.log(m3.createJSONForAnalysis(i3))
//console.log(getRandomInt(2000, 5000))
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const app = yield sdk_1.Actyx.of(factory_protocol_1.manifest);
        const tags = factory_protocol_1.Composition.tagWithEntityId('factory-1');
        const machine = (0, machine_runner_1.createMachineRunner)(app, tags, i3, undefined);
        try {
            for (var _d = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _d = true) {
                _c = machine_1_1.value;
                _d = false;
                const state = _c;
                console.log("state is: ", state);
                const s = state.cast();
                for (var c in s.commands()) {
                    var cmds = s.commands();
                    if (c === 'close') {
                        setTimeout(() => {
                            var _a, _b, _c;
                            const canClose = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.commandsAvailable();
                            if (canClose) {
                                var s1 = (_c = (_b = machine.get()) === null || _b === void 0 ? void 0 : _b.cast()) === null || _c === void 0 ? void 0 : _c.commands();
                                s1.close();
                            }
                            /* if ((await machine.peekNext()).done) {
                                console.log("done")
                                cmds?.close()
                            } else {
                                console.log("not done")
                            } */
                            /* const whenOn = (await machine.peekNext()).done?//.as(peekValue)
                            if (whenOn) {
                                cmds?.close()
                            } */
                        }, (0, factory_protocol_1.getRandomInt)(2000, 5000));
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
