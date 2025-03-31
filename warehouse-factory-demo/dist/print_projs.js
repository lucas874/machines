"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const protocol_1 = require("./protocol");
for (var p of protocol_1.all_projections) {
    console.log(JSON.stringify(p));
    console.log();
    console.log("$$$$");
    console.log();
}
