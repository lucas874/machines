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
const protocol_1 = require("./protocol");
function main() {
    return __awaiter(this, void 0, void 0, function* () {
        const app = yield sdk_1.Actyx.of(protocol_1.manifest);
        const tags = protocol_1.protocol.tagWithEntityId('robot-1');
        yield app.publish(tags.apply(protocol_1.Events.NeedsWater.make({})));
        console.log('Publishing NeedsWater');
        yield app.publish(tags.apply(protocol_1.Events.HasWater.make({})));
        console.log('Publishing HasWater');
        app.dispose();
    });
}
main();
