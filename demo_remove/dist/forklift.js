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
const [m3, i3] = factory_protocol_1.Composition.extendMachine("FL", mAnalysisResource, factory_protocol_1.Events.allEvents, [forklift, exports.s0], fMap);
//console.log(m3.createJSONForAnalysis(i3))
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const app = yield sdk_1.Actyx.of(factory_protocol_1.manifest);
        const tags = factory_protocol_1.Composition.tagWithEntityId('factory-1');
        const machine = (0, machine_runner_1.createMachineRunner)(app, tags, i3, undefined);
        try {
            //var hasRequested = false
            //var isDone = false
            for (var _d = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _d = true) {
                _c = machine_1_1.value;
                _d = false;
                const state = _c;
                console.log("state is: ", state);
                /* if (isDone) {
                    console.log("shutting down")
                    break
                } */
                const s = state.cast();
                for (var c in s.commands()) {
                    var cmds = s.commands();
                    if (c === 'get') {
                        setTimeout(() => {
                            cmds === null || cmds === void 0 ? void 0 : cmds.get();
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
