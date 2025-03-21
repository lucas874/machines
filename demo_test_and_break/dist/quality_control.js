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
// Using the machine runner DSL an implmentation of quality control robot in Gquality is:
const qcr = factory_protocol_1.Composition.makeMachine('QCR');
exports.s0 = qcr.designEmpty('s0')
    .command("observe", [factory_protocol_1.Events.observing], (s, _) => {
    console.log("began observing");
    return [factory_protocol_1.Events.observing.make({})];
})
    .finish();
exports.s1 = qcr.designEmpty('s1').finish();
exports.s2 = qcr.designState('s2').withPayload()
    .command("test", [factory_protocol_1.Events.report], (s, _) => {
    console.log("the newly built", s.self.modelName, " is", s.self.decision);
    return [factory_protocol_1.Events.report.make({ modelName: s.self.modelName, decision: s.self.decision })];
})
    .finish();
exports.s0.react([factory_protocol_1.Events.observing], exports.s1, (_) => exports.s1.make());
exports.s1.react([factory_protocol_1.Events.car], exports.s2, (_, e) => {
    console.log("received a ", e.payload.modelName);
    if (e.payload.part !== 'broken part') {
        return exports.s2.make({ modelName: e.payload.modelName, decision: "ok" });
    }
    else {
        return exports.s2.make({ modelName: e.payload.modelName, decision: "notOk" });
    }
});
// Projection of Gwarehouse || Gfactory || Gquality over QCR
const result_projection_info = (0, machine_check_1.projectionAndInformation)(factory_protocol_1.interfacing_swarms, factory_protocol_1.subs, "QCR");
if (result_projection_info.type == 'ERROR')
    throw new Error('error getting projection');
const projection_info = result_projection_info.data;
console.log(projection_info);
// Command map
const cMap = new Map();
cMap.set(factory_protocol_1.Events.report.type, (s, _) => {
    console.log("the newly built", s.self.modelName, " is", s.self.decision);
    return { modelName: s.self.modelName, decision: s.self.decision };
});
//return [Events.report.make({modelName: s.self.modelName, decision: s.self.decision})]})
cMap.set(factory_protocol_1.Events.observing.type, (s, e) => {
    console.log("began observing");
    console.log("typeof {}: ", typeof ({}));
    return {};
});
//return [Events.observing.make({})]})
// Reaction map
const rMap = new Map();
const carReaction = {
    genPayloadFun: (_, e) => {
        console.log("received a ", e.payload.modelName);
        if (e.payload.part !== 'broken part') {
            return { modelName: e.payload.modelName, decision: "ok" };
        }
        else {
            return { modelName: e.payload.modelName, decision: "notOk" };
        }
    }
};
rMap.set(factory_protocol_1.Events.car.type, carReaction);
const fMap = { commands: cMap, reactions: rMap, initialPayloadType: undefined };
// Extended machine
const [m3, i3] = factory_protocol_1.Composition.extendMachineBT("QCR", projection_info, factory_protocol_1.Events.allEvents, fMap, qcr);
const checkProjResult = (0, machine_check_1.checkComposedProjection)(factory_protocol_1.interfacing_swarms, factory_protocol_1.subs, "QCR", m3.createJSONForAnalysis(i3));
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
                console.log("quality control robot. state is:", state.type);
                if (state.payload !== undefined) {
                    console.log("state payload is:", state.payload);
                }
                console.log();
                const s = state.cast();
                for (var c in s.commands()) {
                    if (c === 'observe') {
                        setTimeout(() => {
                            var _a, _b;
                            var s1 = (_b = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.cast()) === null || _b === void 0 ? void 0 : _b.commands();
                            if (Object.keys(s1 || {}).includes('observe')) {
                                s1.observe();
                            }
                        }, (0, factory_protocol_1.getRandomInt)(2000, 5000));
                        break;
                    }
                    if (c === 'test') {
                        setTimeout(() => {
                            var _a, _b;
                            var s1 = (_b = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.cast()) === null || _b === void 0 ? void 0 : _b.commands();
                            if (Object.keys(s1 || {}).includes('test')) {
                                s1.test();
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
