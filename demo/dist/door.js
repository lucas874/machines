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
// Using the machine runner DSL an implmentation of door in Gwarehouse is:
const door = factory_protocol_1.Composition.makeMachine('D');
exports.s0 = door.designEmpty('s0')
    .command('close', [factory_protocol_1.Events.time], () => {
    var dateString = new Date().toLocaleString();
    console.log("closed warehouse at:", dateString);
    return [factory_protocol_1.Events.time.make({ timeOfDay: dateString })];
})
    .finish();
exports.s1 = door.designEmpty('s1').finish();
exports.s2 = door.designEmpty('s2').finish();
exports.s0.react([factory_protocol_1.Events.partID], exports.s1, (_) => exports.s1.make());
exports.s1.react([factory_protocol_1.Events.part], exports.s0, (_) => exports.s0.make());
exports.s0.react([factory_protocol_1.Events.time], exports.s2, (_) => exports.s2.make());
// Projection of Gwarehouse || Gfactory || Gquality over D
const projectionInfoResult = (0, machine_check_1.projectionAndInformation)(factory_protocol_1.interfacing_swarms, factory_protocol_1.subs, "D");
if (projectionInfoResult.type == 'ERROR')
    throw new Error('error getting projection');
const projectionInfo = projectionInfoResult.data;
//console.log("projection info: ", projectionInfo)
// Adapted machine
const [doorAdapted, s0_] = factory_protocol_1.Composition.adaptMachine("D", projectionInfo, factory_protocol_1.Events.allEvents, exports.s0);
const checkProjResult = (0, machine_check_1.checkComposedProjection)(factory_protocol_1.interfacing_swarms, factory_protocol_1.subs, "D", doorAdapted.createJSONForAnalysis(s0_));
if (checkProjResult.type == 'ERROR')
    throw new Error(checkProjResult.errors.join(", "));
// Run the adapted machine
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const app = yield sdk_1.Actyx.of(factory_protocol_1.manifest);
        const tags = factory_protocol_1.Composition.tagWithEntityId('factory-1');
        //const machine = createMachineRunner(app, tags, s0, undefined)
        const machine = (0, machine_runner_1.createMachineRunnerBT)(app, tags, s0_, undefined, projectionInfo.succeeding_non_branching_joining, projectionInfo.branching_joining);
        try {
            for (var _d = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _d = true) {
                _c = machine_1_1.value;
                _d = false;
                const state = _c;
                console.log("door. state is:", state.type);
                if (state.payload !== undefined) {
                    console.log("state payload is:", state.payload);
                }
                console.log();
                const s = state.cast();
                for (var c in s.commands()) {
                    if (c === 'close') {
                        setTimeout(() => {
                            var _a, _b;
                            var s1 = (_b = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.cast()) === null || _b === void 0 ? void 0 : _b.commands();
                            if (Object.keys(s1 || {}).includes('close')) {
                                s1.close();
                            }
                        }, (0, factory_protocol_1.getRandomInt)(5500, 8000));
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
