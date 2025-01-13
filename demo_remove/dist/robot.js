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
exports.WateringPlant = exports.Idle = void 0;
exports.main = main;
const machine_runner_1 = require("@actyx/machine-runner");
const sdk_1 = require("@actyx/sdk");
const protocol_1 = require("./protocol");
const machine = protocol_1.protocol.makeMachine('robot');
exports.Idle = machine.designEmpty('Idle').finish();
exports.WateringPlant = machine.designEmpty('WateringPlant').finish();
exports.Idle.react([protocol_1.Events.NeedsWater], exports.WateringPlant, (_) => exports.WateringPlant.make());
exports.WateringPlant.react([protocol_1.Events.HasWater], exports.Idle, (_) => exports.Idle.make());
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        var _a, e_1, _b, _c;
        const sdk = yield sdk_1.Actyx.of(protocol_1.manifest);
        const tags = protocol_1.protocol.tagWithEntityId('robot-1');
        const machine = (0, machine_runner_1.createMachineRunner)(sdk, tags, exports.Idle, undefined);
        try {
            for (var _d = true, machine_1 = __asyncValues(machine), machine_1_1; machine_1_1 = yield machine_1.next(), _a = machine_1_1.done, !_a; _d = true) {
                _c = machine_1_1.value;
                _d = false;
                const state = _c;
                console.log(state);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (!_d && !_a && (_b = machine_1.return)) yield _b.call(machine_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
    });
}
main();
