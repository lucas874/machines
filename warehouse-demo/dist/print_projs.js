"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const warehouse_protocol_1 = require("./warehouse_protocol");
for (var p of warehouse_protocol_1.all_projections) {
    console.log(JSON.stringify(p));
    console.log();
    console.log("$$$$");
    console.log();
}
