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
const sdk_1 = require("@actyx/sdk");
const machine_runner_1 = require("@actyx/machine-runner");
const warehouse_protocol_1 = require("./warehouse_protocol");
const machine_check_1 = require("@actyx/machine-check");
/*

Using the machine runner DSL an implmentation of door in Gwarehouse is:

const door = Composition.makeMachine('D')
export const s0 = door.designEmpty('s0')
    .command('close', [Events.time], () => {
        var dateString = new Date().toLocaleString();
        console.log("closed warehouse at:", dateString);
        return [Events.time.make({timeOfDay: dateString})]})
    .finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_) => s1.make())
s1.react([Events.part], s0, (_) => s0.make())
s0.react([Events.time], s2, (_) => s2.make())
*/
/* for (var p of all_projections) {
    console.log(JSON.stringify(p))
    console.log("$$$$")
} */
// Projection of Gwarehouse || Gfactory || Gquality over D
const result_projection = (0, machine_check_1.projectCombineMachines)(warehouse_protocol_1.interfacing_swarms, warehouse_protocol_1.subs, "D");
if (result_projection.type == 'ERROR')
    throw new Error('error getting projection');
const projection = result_projection.data;
// Command map
const cMap = new Map();
cMap.set(warehouse_protocol_1.Events.time.type, () => {
    var dateString = new Date().toLocaleString();
    console.log("closed warehouse at:", dateString);
    return [warehouse_protocol_1.Events.time.make({ timeOfDay: dateString })];
});
// Reaction map
const rMap = new Map();
const fMap = { commands: cMap, reactions: rMap, initialPayloadType: undefined };
// Extended machine
const [m3, i3] = warehouse_protocol_1.Composition.extendMachine("D", projection, warehouse_protocol_1.Events.allEvents, fMap);
const checkProjResult = (0, machine_check_1.checkComposedProjection)(warehouse_protocol_1.interfacing_swarms, warehouse_protocol_1.subs, "D", m3.createJSONForAnalysis(i3));
if (checkProjResult.type == 'ERROR')
    throw new Error(checkProjResult.errors.join(", "));
// Run the extended machine
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const app = yield sdk_1.Actyx.of(warehouse_protocol_1.manifest);
        const tags = warehouse_protocol_1.Composition.tagWithEntityId('factory-1');
        const machine = (0, machine_runner_1.createMachineRunner)(app, tags, i3, undefined);
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
                            if (Object.keys(s1).includes('close')) {
                                s1.close();
                            }
                        }, (0, warehouse_protocol_1.getRandomInt)(5000, 8000));
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
