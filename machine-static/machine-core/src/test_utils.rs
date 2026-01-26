use crate::types::typescript_types::{InterfacingProtocols, SwarmProtocolType};

use tracing_subscriber::{EnvFilter, fmt, fmt::format::FmtSpan};
pub fn setup_logger() {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .try_init()
        .ok();
}

pub fn get_proto1() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                { "source": "2", "target": "0", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                { "source": "0", "target": "3", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn get_proto2() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                { "source": "1", "target": "2", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                { "source": "2", "target": "3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn get_proto3() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["report1"], "role": "TR" } },
                { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report2"], "role": "TR" } },
                { "source": "3", "target": "4", "label": { "cmd": "accept", "logType": ["ok"], "role": "QCR" } },
                { "source": "3", "target": "4", "label": { "cmd": "reject", "logType": ["notOk"], "role": "QCR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn get_proto31() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "observe1", "logType": ["report1"], "role": "QCR" } },
                { "source": "1", "target": "2", "label": { "cmd": "observe2", "logType": ["report2"], "role": "QCR" } },
                { "source": "2", "target": "3", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                { "source": "3", "target": "4", "label": { "cmd": "assess", "logType": ["report3"], "role": "QCR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn get_proto32() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["observing"], "role": "QCR" } },
                { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report"], "role": "QCR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn get_proto_4() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_ir_0", "logType": ["e_ir_0"], "role": "IR" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir_1", "logType": ["e_ir_1"], "role": "IR" } },
                { "source": "2", "target": "1", "label": { "cmd": "c_r0_0", "logType": ["e_r0_0"], "role": "R0" } },
                { "source": "1", "target": "3", "label": { "cmd": "c_r0_1", "logType": ["e_r0_1"], "role": "R0" } }
            ]
        }"#,
    )
    .unwrap()
}
// get_proto_4 from test module of composition_machine
fn get_proto_41() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "observe", "logType": ["observing"], "role": "QCR" } },
                { "source": "1", "target": "2", "label": { "cmd": "build", "logType": ["car"], "role": "F" } },
                { "source": "2", "target": "3", "label": { "cmd": "test", "logType": ["report"], "role": "QCR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn get_proto_5() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_ir_0", "logType": ["e_ir_0"], "role": "IR" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_r1_0", "logType": ["e_r1_0"], "role": "R1" } },
                { "source": "2", "target": "3", "label": { "cmd": "c_ir_1", "logType": ["e_ir_1"], "role": "IR" } }
            ]
        }"#,
    )
    .unwrap()
}

// two event types in close, request appears multiple times, get emits no events
pub fn get_malformed_proto1() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                { "source": "1", "target": "2", "label": { "cmd": "get", "logType": [], "role": "FL" } },
                { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["part"], "role": "T" } },
                { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time", "time2"], "role": "D" } }
            ]
        }"#,
    )
    .unwrap()
}

// initial state state unreachable
pub fn get_malformed_proto2() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["partID"], "role": "T" } }
            ]
        }"#,
    )
    .unwrap()
}

// all states not reachable
pub fn get_malformed_proto3() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                { "source": "2", "target": "3", "label": { "cmd": "deliver", "logType": ["part"], "role": "T" } },
                { "source": "4", "target": "5", "label": { "cmd": "build", "logType": ["car"], "role": "F" } }
            ]
        }"#,
    )
    .unwrap()
}

// pos event type associated with multiple commands and nondeterminism at 0.
// No terminal state can be reached from any state -- OK according to confusion freeness
pub fn get_confusionful_proto1() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                { "source": "0", "target": "0", "label": { "cmd": "request", "logType": ["partID"], "role": "T" } },
                { "source": "1", "target": "2", "label": { "cmd": "get", "logType": ["pos"], "role": "FL" } },
                { "source": "2", "target": "0", "label": { "cmd": "request", "logType": ["pos"], "role": "T" } },
                { "source": "0", "target": "0", "label": { "cmd": "close", "logType": ["time"], "role": "D" } }
            ]
        }"#,
    )
    .unwrap()
}
// No terminal state can be reached from any state -- OK according to confusion freeness
pub fn get_some_nonterminating_proto() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "a", "logType": ["a"], "role": "a" } },
                { "source": "0", "target": "2", "label": { "cmd": "c", "logType": ["c"], "role": "c" } },
                { "source": "2", "target": "3", "label": { "cmd": "b", "logType": ["b"], "role": "b" } },
                { "source": "1", "target": "4", "label": { "cmd": "d", "logType": ["d"], "role": "d" } },
                { "source": "4", "target": "5", "label": { "cmd": "e", "logType": ["e"], "role": "e" } },
                { "source": "5", "target": "1", "label": { "cmd": "f", "logType": ["f"], "role": "f" } }
            ]
        }"#,
    )
    .unwrap()
}

pub fn pattern_4_proto_0() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_r0", "logType": ["e_r0"], "role": "R0" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn pattern_4_proto_1() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_r1", "logType": ["e_r1"], "role": "R1" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn pattern_4_proto_2() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_r2", "logType": ["e_r2"], "role": "R2" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn pattern_4_proto_3() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_r3", "logType": ["e_r3"], "role": "R3" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn pattern_4_proto_4() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_r4", "logType": ["e_r4"], "role": "R4" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir", "logType": ["e_ir"], "role": "IR" } }
            ]
        }"#,
    )
    .unwrap()
}

pub fn ref_pat_proto_0() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_ir0_0", "logType": ["e_ir0_0"], "role": "IR0" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir0_1", "logType": ["e_ir0_1"], "role": "IR0" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn ref_pat_proto_1() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_ir0_0", "logType": ["e_ir0_0"], "role": "IR0" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_ir1_0", "logType": ["e_ir1_0"], "role": "IR1" } },
                { "source": "2", "target": "3", "label": { "cmd": "c_ir1_1", "logType": ["e_ir1_1"], "role": "IR1" } },
                { "source": "3", "target": "4", "label": { "cmd": "c_rb", "logType": ["e_rb"], "role": "RB" } },
                { "source": "4", "target": "5", "label": { "cmd": "c_ir0_1", "logType": ["e_ir0_1"], "role": "IR0" } },
                { "source": "1", "target": "6", "label": { "cmd": "c_ra", "logType": ["e_ra"], "role": "RA" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn ref_pat_proto_2() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "c_ir1_0", "logType": ["e_ir1_0"], "role": "IR1" } },
                { "source": "1", "target": "2", "label": { "cmd": "c_rc", "logType": ["e_rc"], "role": "RC" } },
                { "source": "2", "target": "3", "label": { "cmd": "c_ir1_1", "logType": ["e_ir1_1"], "role": "IR1" } }
            ]
        }"#,
    )
    .unwrap()
}

pub fn get_interfacing_swarms_5() -> InterfacingProtocols {
    InterfacingProtocols(vec![get_proto_4(), get_proto_5()])
}

pub fn get_ref_pat_protos() -> InterfacingProtocols {
    InterfacingProtocols(vec![
        ref_pat_proto_0(),
        ref_pat_proto_1(),
        ref_pat_proto_2(),
    ])
}

pub fn get_interfacing_swarms_1() -> InterfacingProtocols {
    InterfacingProtocols(vec![get_proto1(), get_proto2()])
}

pub fn get_interfacing_swarms_2() -> InterfacingProtocols {
    InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto3()])
}

pub fn get_interfacing_swarms_3() -> InterfacingProtocols {
    InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto31()])
}

pub fn get_interfacing_swarms_4() -> InterfacingProtocols {
    InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto32()])
}

pub fn get_interfacing_swarms_pat_4() -> InterfacingProtocols {
    InterfacingProtocols(vec![
        pattern_4_proto_0(),
        pattern_4_proto_1(),
        pattern_4_proto_2(),
        pattern_4_proto_3(),
        pattern_4_proto_4(),
    ])
}

// get_interfacing_swarms_3 from composition_machine.rs
pub fn get_interfacing_swarms_3_machine() -> InterfacingProtocols {
    InterfacingProtocols(vec![get_proto1(), get_proto2(), get_proto_41()])
}

pub fn get_looping_proto_1() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                { "source": "0", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R1" } },
                { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R2" } },
                { "source": "4", "target": "2", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R1" } }
            ]
        }"#,
    )
    .unwrap()
}
pub fn get_looping_proto_2() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                { "source": "0", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                { "source": "4", "target": "2", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } }
            ]
        }"#,
    )
    .unwrap()
}

pub fn get_looping_proto_3() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                { "source": "0", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                { "source": "4", "target": "2", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                { "source": "1", "target": "5", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R5" } },
                { "source": "5", "target": "6", "label": { "cmd": "cmd_g", "logType": ["g"], "role": "R6" } },
                { "source": "6", "target": "7", "label": { "cmd": "cmd_h", "logType": ["h"], "role": "R6" } },
                { "source": "7", "target": "1", "label": { "cmd": "cmd_i", "logType": ["i"], "role": "R7" } }
            ]
        }"#,
    )
    .unwrap()
}

pub fn get_looping_proto_4() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                { "source": "1", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                { "source": "3", "target": "4", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                { "source": "4", "target": "5", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                { "source": "5", "target": "6", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R6" } },
                { "source": "6", "target": "7", "label": { "cmd": "cmd_g", "logType": ["g"], "role": "R7" } },
                { "source": "7", "target": "2", "label": { "cmd": "cmd_h", "logType": ["h"], "role": "R8" } }
            ]
        }"#,
    )
    .unwrap()
}

pub fn get_looping_proto_5() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                { "source": "1", "target": "2", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                { "source": "2", "target": "3", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                { "source": "3", "target": "0", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                { "source": "0", "target": "4", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                { "source": "4", "target": "5", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R6" } },
                { "source": "5", "target": "6", "label": { "cmd": "cmd_g", "logType": ["g"], "role": "R7" } },
                { "source": "6", "target": "0", "label": { "cmd": "cmd_h", "logType": ["h"], "role": "R8" } }
            ]
        }"#,
    )
    .unwrap()
}

pub fn get_looping_proto_6() -> SwarmProtocolType {
    serde_json::from_str::<SwarmProtocolType>(
        r#"{
            "initial": "0",
            "transitions": [
                { "source": "0", "target": "1", "label": { "cmd": "cmd_a", "logType": ["a"], "role": "R1" } },
                { "source": "1", "target": "0", "label": { "cmd": "cmd_b", "logType": ["b"], "role": "R2" } },
                { "source": "1", "target": "2", "label": { "cmd": "cmd_c", "logType": ["c"], "role": "R3" } },
                { "source": "2", "target": "3", "label": { "cmd": "cmd_d", "logType": ["d"], "role": "R4" } },
                { "source": "3", "target": "4", "label": { "cmd": "cmd_e", "logType": ["e"], "role": "R5" } },
                { "source": "4", "target": "0", "label": { "cmd": "cmd_f", "logType": ["f"], "role": "R6" } }
            ]
        }"#,
    )
    .unwrap()
}
