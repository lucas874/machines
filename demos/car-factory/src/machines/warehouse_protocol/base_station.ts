import { Events, Composition, WarehouseProtocol } from '../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

type GiveGuidancePayload = { item: string, to: string }

// Using the machine runner DSL an implmentation of body assembler in the steel press protocol:
export const baseStation = Composition.makeMachine(WarehouseProtocol.baseStationRole)
export const s0 = baseStation.designEmpty('s0').finish()
export const s1 = baseStation.designEmpty('s1').finish()
export const s2 = baseStation.designState('s2')
    .withPayload<GiveGuidancePayload>()
    .command(WarehouseProtocol.cmdGiveGuidance, [Events.giveGuidance], (_) =>
        [Events.giveGuidance.make({ directions: ["LEFT", "RIGHT", "LEFT"] })])
    .finish()

s0.react([Events.bid], s0, (_) => { return s0.make() })
s0.react([Events.selected], s1, (_) => { return s1.make() })
s1.react([Events.itemPickupSmart], s0, (_) => { return s0.make() })
s1.react([Events.requestGuidance], s2, (_, event) => { return s2.make({ item: event.payload.item, to: event.payload.to }) })
s2.react([Events.giveGuidance], s0, () => { return s0.make() })
// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([WarehouseProtocol.protocol], WarehouseProtocol.subscriptions, WarehouseProtocol.baseStationRole, baseStation.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))