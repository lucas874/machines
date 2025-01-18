"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const factory_protocol_1 = require("./factory_protocol");
for (var p of factory_protocol_1.all_projections) {
    console.log(JSON.stringify(p));
    console.log();
}
