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
exports.s0 = transporter.designState('s0').withPayload()
    .command('request', [warehouse_protocol_1.Events.partID], (s, e) => {
    var id = s.self.id;
    console.log("requesting a", id);
    return [warehouse_protocol_1.Events.partID.make({ id: id })];
})
    .finish();
exports.s1 = transporter.designEmpty('s1').finish();
exports.s2 = transporter.designState('s2').withPayload()
    .command('deliver', [warehouse_protocol_1.Events.part], (s, e) => {
    console.log("delivering a", s.self.part);
    return [warehouse_protocol_1.Events.part.make({ part: s.self.part })];
})
    .finish();
exports.s3 = transporter.designEmpty('s3').finish();
exports.s0.react([warehouse_protocol_1.Events.partID], exports.s1, (_) => exports.s1.make());
exports.s0.react([warehouse_protocol_1.Events.time], exports.s3, (_) => exports.s3.make());
exports.s1.react([warehouse_protocol_1.Events.position], exports.s2, (_, e) => {
    console.log("e is: ", e);
    console.log("got a ", e.payload.part);
    return { part: e.payload.part };
});
exports.s2.react([warehouse_protocol_1.Events.part], exports.s0, (_, e) => { console.log("e is: ", e); return exports.s0.make({ id: "" }); });
// Projection of Gwarehouse || Gfactory || Gquality over D
const result_projection = (0, machine_check_1.projectCombineMachines)(warehouse_protocol_1.interfacing_swarms, warehouse_protocol_1.subs, "T");
if (result_projection.type == 'ERROR')
    throw new Error('error getting projection');
const projection = result_projection.data;
// Command map
const cMap = new Map();
cMap.set(warehouse_protocol_1.Events.partID.type, (s, e) => {
    s.self.id = s.self.id === undefined ? parts[Math.floor(Math.random() * parts.length)] : s.self.id;
    var id = s.self.id;
    console.log("requesting a", id);
    console.log("in command, s is: ", s);
    return { id: id };
});
//return [Events.partID.make({id: id})]})
cMap.set(warehouse_protocol_1.Events.part.type, (s, e) => {
    console.log("delivering a", s.self.part);
    return { part: s.self.part };
});
//return [Events.part.make({part: s.self.part})] })
// Reaction map
const rMap = new Map();
const positionReaction = {
    genPayloadFun: (s, e) => {
        //console.log("e is", e); console.log("s is: :", s);
        return { part: e.payload.part };
    }
};
rMap.set(warehouse_protocol_1.Events.position.type, positionReaction);
const partIDReaction = {
    genPayloadFun: (s, e) => {
        //console.log("e is", e); console.log("s is: :", s);
        return {};
    }
};
rMap.set(warehouse_protocol_1.Events.partID.type, partIDReaction);
/* const partReaction : ProjMachine.ReactionEntry = {
  genPayloadFun: (s, e) => {
    console.log("part reaction"); console.log("e is", e); console.log("s is: :", s) }
}
rMap.set(Events.part.type, partReaction) */
// hacky. we use the return type of this function to set the payload type of initial state and any other state enabling same commands as in initial
const initialPayloadType = {
    genPayloadFun: () => { return { part: "" }; }
};
const fMap = { commands: cMap, reactions: rMap, initialPayloadType: initialPayloadType };
console.log(projection);
// Extended machine
const [m3, i3] = warehouse_protocol_1.Composition.extendMachineBT("T", projection, warehouse_protocol_1.Events.allEvents, fMap, new Set([warehouse_protocol_1.Events.partID.type, warehouse_protocol_1.Events.time.type]));
const checkProjResult = (0, machine_check_1.checkComposedProjection)(warehouse_protocol_1.interfacing_swarms, warehouse_protocol_1.subs, "T", m3.createJSONForAnalysis(i3));
if (checkProjResult.type == 'ERROR')
    throw new Error(checkProjResult.errors.join(", "));
// Run the extended machine
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        var _d, _e;
        const app = yield sdk_1.Actyx.of(warehouse_protocol_1.manifest);
        const tags = warehouse_protocol_1.Composition.tagWithEntityId('factory-1');
        const machine = (0, machine_runner_1.createMachineRunner)(app, tags, i3, { lbj: null, payload: { id: parts[Math.floor(Math.random() * parts.length)] } });
        try {
            for (var _f = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _f = true) {
                _c = machine_1_1.value;
                _f = false;
                const state = _c;
                console.log("transporter. state is:", state.type);
                //if (state.payload !== undefined) {
                //  console.log("state payload is:", state.payload)
                //}
                //console.log("transporter state is: ", state)
                //console.log()
                const s = state.cast();
                for (var c in s.commands()) {
                    if (c === 'request') {
                        //setTimeout(() => {
                        var s1 = (_e = (_d = machine.get()) === null || _d === void 0 ? void 0 : _d.cast()) === null || _e === void 0 ? void 0 : _e.commands();
                        if (Object.keys(s1 || {}).includes('request')) {
                            s1.request();
                        }
                        // }, getRandomInt(500, 5000))
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
                if (!_f && !_a && (_b = machine_1.return)) yield _b.call(machine_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
        app.dispose();
    });
}
main();
