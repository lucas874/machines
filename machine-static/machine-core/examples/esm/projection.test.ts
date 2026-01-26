import { describe, expect, it } from '@jest/globals'
import { WarehouseFactory } from './proto.js'
import { MachineType, Subscriptions, DataResult, overapproxWFSubscriptions, projectionInformation } from '../..'


const machineFL: MachineType = {
    initial: "0",
    transitions: [
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypePartReq },
            source: "0",
            target: "1",
        },
        {
            label: { tag: "Execute", cmd: WarehouseFactory.cmdGet, logType: [WarehouseFactory.eventTypePos] },
            source: "1",
            target: "1",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypePos },
            source: "1",
            target: "2",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypePartReq },
            source: "2",
            target: "1",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypeClosingTime },
            source: "2",
            target: "3",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypeClosingTime },
            source: "0",
            target: "3",
        },
    ]
};

const expectedAdaptedFL: MachineType = {
    initial: "(0 || { { 0 } }) || { { 0 } }",
    transitions: [
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypeClosingTime },
            source: "(0 || { { 0 } }) || { { 0 } }",
            target: "(3 || { { 3 } }) || { { 0 } }",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypePartReq },
            source: "(0 || { { 0 } }) || { { 0 } }",
            target: "(1 || { { 1 } }) || { { 1 } }",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypePos },
            source: "(1 || { { 1 } }) || { { 1 } }",
            target: "(2 || { { 2 } }) || { { 1 } }",
        },
        {
            label: { tag: "Execute", cmd: WarehouseFactory.cmdGet, logType: [WarehouseFactory.eventTypePos] },
            source: "(1 || { { 1 } }) || { { 1 } }",
            target: "(1 || { { 1 } }) || { { 1 } }",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypePartOk },
            source: "(2 || { { 2 } }) || { { 1 } }",
            target: "(2 || { { 0 } }) || { { 2 } }",
        },
        {
            label: { tag: "Input", eventType: WarehouseFactory.eventTypeClosingTime },
            source: "(2 || { { 0 } }) || { { 2 } }",
            target: "(3 || { { 3 } }) || { { 2 } }",
        },
    ]
};

const expectedBranchMap = {
    partReq: [WarehouseFactory.eventTypePos, WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime],
    pos: [WarehouseFactory.eventTypePartOk, WarehouseFactory.eventTypeClosingTime],
    partOk: [WarehouseFactory.eventTypeClosingTime],
    closingTime: [],
}

const expectedUpdatingEventTypes = [WarehouseFactory.eventTypePartReq, WarehouseFactory.eventTypeClosingTime]

const expectedProjToMachineStates: Record<string, string[]> = {}
expectedProjToMachineStates["(0 || { { 0 } }) || { { 0 } }"] = ["0"]
expectedProjToMachineStates["(3 || { { 3 } }) || { { 0 } }"] = ["3"]
expectedProjToMachineStates["(1 || { { 1 } }) || { { 1 } }"] = ["1"]
expectedProjToMachineStates["(2 || { { 2 } }) || { { 1 } }"] = ["2"]
expectedProjToMachineStates["(2 || { { 0 } }) || { { 2 } }"] = ["2"]
expectedProjToMachineStates["(3 || { { 3 } }) || { { 2 } }"] = ["3"]

describe('projection information for FL from warehouse adapted to warehouse || factory', () => {
    const subscriptionResult: DataResult<Subscriptions> = overapproxWFSubscriptions(WarehouseFactory.protocols, {}, 'TwoStep')
    if (subscriptionResult.type === 'ERROR') throw new Error('expected subscription generation result to be ok')
    const subscriptions: Subscriptions = subscriptionResult.data
    it('projection information', () => {
        const result = projectionInformation(WarehouseFactory.roleFL, WarehouseFactory.protocols, 0, subscriptions, machineFL, true)
        expect(result.type).toBe('OK')
        if (result.type === 'ERROR') throw new Error('expected projection information generation result to be ok')
        
            expectedAdaptedFL.transitions.sort()
        result.data.projection.transitions.sort()
        expect(result.data.projection).toEqual(expectedAdaptedFL)

        expect(result.data.branches[WarehouseFactory.eventTypePartReq].sort()).toEqual(expectedBranchMap[WarehouseFactory.eventTypePartReq].sort())
        expect(result.data.branches[WarehouseFactory.eventTypePos].sort()).toEqual(expectedBranchMap[WarehouseFactory.eventTypePos].sort())
        expect(result.data.branches[WarehouseFactory.eventTypePartOk].sort()).toEqual(expectedBranchMap[WarehouseFactory.eventTypePartOk].sort())
        expect(result.data.branches[WarehouseFactory.eventTypeClosingTime].sort()).toEqual(expectedBranchMap[WarehouseFactory.eventTypeClosingTime].sort())
        
        expect(result.data.specialEventTypes.sort()).toEqual(expectedUpdatingEventTypes.sort())

        expect(result.data.projToMachineStates["(0 || { { 0 } }) || { { 0 } }"]).toEqual(expectedProjToMachineStates["(0 || { { 0 } }) || { { 0 } }"])
        expect(result.data.projToMachineStates["(3 || { { 3 } }) || { { 0 } }"]).toEqual(expectedProjToMachineStates["(3 || { { 3 } }) || { { 0 } }"])
        expect(result.data.projToMachineStates["(1 || { { 1 } }) || { { 1 } }"]).toEqual(expectedProjToMachineStates["(1 || { { 1 } }) || { { 1 } }"])
        expect(result.data.projToMachineStates["(2 || { { 2 } }) || { { 1 } }"]).toEqual(expectedProjToMachineStates["(2 || { { 2 } }) || { { 1 } }"])
        expect(result.data.projToMachineStates["(2 || { { 0 } }) || { { 2 } }"]).toEqual(expectedProjToMachineStates["(2 || { { 0 } }) || { { 2 } }"])
        expect(result.data.projToMachineStates["(3 || { { 3 } }) || { { 2 } }"]).toEqual(expectedProjToMachineStates["(3 || { { 3 } }) || { { 2 } }"])
    })
})