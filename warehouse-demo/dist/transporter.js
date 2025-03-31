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
exports.s3 = exports.s2 = exports.s1 = exports.s0 = void 0;
const sdk_1 = require("@actyx/sdk");
const machine_runner_1 = require("@actyx/machine-runner");
const warehouse_protocol_1 = require("./warehouse_protocol");
const machine_check_1 = require("@actyx/machine-check");
const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler'];
// Using the machine runner DSL an implmentation of transporter in Gwarehouse is:
const transporter = warehouse_protocol_1.Composition.makeMachine('T');
exports.s0 = transporter.designEmpty('s0')
    .command('request', [warehouse_protocol_1.Events.partReq], (s, e) => {
    var id = parts[Math.floor(Math.random() * parts.length)];
    console.log("requesting a", id);
    return [warehouse_protocol_1.Events.partReq.make({ id: id })];
})
    .finish();
exports.s1 = transporter.designEmpty('s1').finish();
exports.s2 = transporter.designState('s2').withPayload()
    .command('deliver', [warehouse_protocol_1.Events.partOK], (s, e) => {
    console.log("delivering a", s.self.part);
    return [warehouse_protocol_1.Events.partOK.make({ part: s.self.part })];
})
    .finish();
exports.s3 = transporter.designEmpty('s3').finish();
exports.s0.react([warehouse_protocol_1.Events.partReq], exports.s1, (_) => exports.s1.make());
exports.s0.react([warehouse_protocol_1.Events.closingTime], exports.s3, (_) => exports.s3.make());
exports.s1.react([warehouse_protocol_1.Events.pos], exports.s2, (_, e) => {
    console.log("got a ", e.payload.part);
    return { part: e.payload.part };
});
exports.s2.react([warehouse_protocol_1.Events.partOK], exports.s0, (_, e) => { return exports.s0.make(); });
// Projection of Gwarehouse over T
const projectionInfoResult = (0, machine_check_1.projectionAndInformation)(warehouse_protocol_1.interfacing_swarms, warehouse_protocol_1.subs, "T");
if (projectionInfoResult.type == 'ERROR')
    throw new Error('error getting projection');
const projectionInfo = projectionInfoResult.data;
const checkProjResult = (0, machine_check_1.checkComposedProjection)(warehouse_protocol_1.interfacing_swarms, warehouse_protocol_1.subs, "T", transporter.createJSONForAnalysis(exports.s0));
if (checkProjResult.type == 'ERROR')
    throw new Error(checkProjResult.errors.join(", "));
// Run the adapted machine
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const app = yield sdk_1.Actyx.of(warehouse_protocol_1.manifest);
        const tags = warehouse_protocol_1.Composition.tagWithEntityId('warehouse-1');
        const machine = (0, machine_runner_1.createMachineRunnerBT)(app, tags, exports.s0, undefined, projectionInfo.branches, projectionInfo.specialEventTypes);
        try {
            for (var _d = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _d = true) {
                _c = machine_1_1.value;
                _d = false;
                const state = _c;
                console.log("transporter. state is:", state.type);
                if (state.payload !== undefined) {
                    console.log("state payload is:", state.payload);
                }
                console.log();
                const s = state.cast();
                for (var c in s.commands()) {
                    if (c === 'request') {
                        setTimeout(() => {
                            var _a, _b;
                            var s1 = (_b = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.cast()) === null || _b === void 0 ? void 0 : _b.commands();
                            if (Object.keys(s1 || {}).includes('request')) {
                                s1.request();
                            }
                        }, (0, warehouse_protocol_1.getRandomInt)(500, 5000));
                        break;
                    }
                    if (c === 'deliver') {
                        setTimeout(() => {
                            var _a, _b;
                            var s1 = (_b = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.cast()) === null || _b === void 0 ? void 0 : _b.commands();
                            if (Object.keys(s1 || {}).includes('deliver')) {
                                s1.deliver();
                            }
                        }, (0, warehouse_protocol_1.getRandomInt)(500, 8000));
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
