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
Object.defineProperty(exports, "__esModule", { value: true });
const sdk_1 = require("@actyx/sdk");
const warehouse_protocol_1 = require("./warehouse_protocol");
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        const app = yield sdk_1.Actyx.of(warehouse_protocol_1.manifest);
        const tags = warehouse_protocol_1.Composition.tagWithEntityId('factory-1');
        while (true) {
            yield new Promise(f => setTimeout(f, 5000));
            yield app.publish(tags.apply(warehouse_protocol_1.Events.closingTime.makeBT({ timeOfDay: new Date().toLocaleString() }, "invalidPointer")));
            console.log('Publishing time event with invalid lbj pointer');
        }
        app.dispose();
    });
}
main();
