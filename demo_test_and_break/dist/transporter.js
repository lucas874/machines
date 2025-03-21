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
const factory_protocol_1 = require("./factory_protocol");
const machine_check_1 = require("@actyx/machine-check");
// Using the machine runner DSL an implmentation of transporter in Gwarehouse is:
const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler'];
const transporter = factory_protocol_1.Composition.makeMachine('T');
exports.s0 = transporter.designState('s0').withPayload()
    .command('request', [factory_protocol_1.Events.partID], (s, e, t) => {
    console.log("whole s in request is: ", s);
    console.log("e and t: ", e, " ", t); // 1 and 2 when called with request(1, 2, 3, 4), meaning ok that more paramters than specified, but event is not passed as default, which event would this be anyway. only CONTEXT which is called s here
    var id = s.self.id;
    console.log("requesting a", id);
    return [factory_protocol_1.Events.partID.make({ id: id })];
})
    .finish();
exports.s1 = transporter.designEmpty('s1').finish();
exports.s2 = transporter.designState('s2').withPayload()
    .command('deliver', [factory_protocol_1.Events.part], (s, e) => {
    console.log("whole s in deliver is: ", s);
    console.log("delivering a", s.self.part);
    return [factory_protocol_1.Events.part.make({ part: s.self.part })];
})
    .finish();
exports.s3 = transporter.designEmpty('s3').finish();
exports.s0.react([factory_protocol_1.Events.partID], exports.s1, (s, e) => { console.log("event is: ", e); return exports.s1.make(); });
exports.s0.react([factory_protocol_1.Events.time], exports.s3, (_) => exports.s3.make());
exports.s1.react([factory_protocol_1.Events.position], exports.s2, (_, e) => {
    console.log("event is: ", e);
    console.log("got a ", e.payload.part);
    return { part: e.payload.part };
});
// s0.make(...) etc. just creates the state payload!
exports.s2.react([factory_protocol_1.Events.part], exports.s0, (_, e) => { return exports.s0.make({ id: parts[Math.floor(Math.random() * parts.length)] }); });
// s2.react([Events.part], s0, (_, e) => { return s0.make() })
// Projection of Gwarehouse || Gfactory || Gquality over D
const result_projection_info = (0, machine_check_1.projectionAndInformation)(factory_protocol_1.interfacing_swarmswh, factory_protocol_1.subswh, "T");
if (result_projection_info.type == 'ERROR')
    throw new Error('error getting projection');
const projection_info = result_projection_info.data;
//console.log(projection_info)
// Command map
const cMap = new Map();
cMap.set(factory_protocol_1.Events.partID.type, (s, e) => {
    var id = s.self.id;
    console.log("requesting a", id);
    //return {id: id} })
    return [factory_protocol_1.Events.partID.make({ id: id })];
});
cMap.set(factory_protocol_1.Events.part.type, (s, e) => {
    console.log("s is: ", s);
    console.log("delivering a", s.self.part);
    //return {part: s.self.part}})
    return [factory_protocol_1.Events.part.make({ part: s.self.part })];
});
// Reaction map
const rMap = new Map();
const positionReaction = {
    genPayloadFun: (_, e) => { return { part: e.payload.part }; }
};
rMap.set(factory_protocol_1.Events.position.type, positionReaction);
const partReaction = {
    genPayloadFun: (_, e) => { return { id: parts[Math.floor(Math.random() * parts.length)] }; }
};
rMap.set(factory_protocol_1.Events.part.type, partReaction);
// hacky. we use the return type of this function to set the payload type of initial state and any other state enabling same commands as in initial
const initialPayloadType = {
    genPayloadFun: () => { return { part: "" }; }
};
const fMap = { commands: cMap, reactions: rMap, initialPayloadType: initialPayloadType };
// Extended machine
const [m3, i3] = factory_protocol_1.Composition.adaptMachine("T", projection_info, factory_protocol_1.Events.allEvents, exports.s0);
const checkProjResult = (0, machine_check_1.checkComposedProjection)(factory_protocol_1.interfacing_swarmswh, factory_protocol_1.subswh, "T", m3.createJSONForAnalysis(i3));
if (checkProjResult.type == 'ERROR')
    throw new Error(checkProjResult.errors.join(", "));
// Run the extended machine
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const app = yield sdk_1.Actyx.of(factory_protocol_1.manifest);
        const tags = factory_protocol_1.Composition.tagWithEntityId('factory-1');
        //const machine = createMachineRunner(app, tags, s0, {id: parts[Math.floor(Math.random() * parts.length)]})
        const machine = (0, machine_runner_1.createMachineRunnerBT)(app, tags, i3, { id: parts[Math.floor(Math.random() * parts.length)] }, projection_info.succeeding_non_branching_joining, projection_info.branching_joining);
        try {
            for (var _d = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _d = true) {
                _c = machine_1_1.value;
                _d = false;
                const state = _c;
                console.log("transporter. state is:", state.type);
                console.log("transporter whole state: ", state);
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
                        }, (0, factory_protocol_1.getRandomInt)(2000, 5000));
                        break;
                    }
                    if (c === 'deliver') {
                        setTimeout(() => {
                            var _a, _b;
                            var s1 = (_b = (_a = machine.get()) === null || _a === void 0 ? void 0 : _a.cast()) === null || _b === void 0 ? void 0 : _b.commands();
                            if (Object.keys(s1 || {}).includes('deliver')) {
                                s1.deliver();
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
