import { Events, Composition, EngineInstallationProtocol, type ItemDeliveryPayload } from './../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

//type RequestEnginePayload = { shape: string }

// Using the machine runner DSL an implmentation of body assembler in the steel press protocol:
export const warehouse = Composition.makeMachine(EngineInstallationProtocol.warehouseRole)
export const s0 = warehouse.designEmpty('s0').finish()
export const s1 = warehouse.designState('s1')
    .withPayload<ItemDeliveryPayload>()
    .command(EngineInstallationProtocol.cmdRequest, [Events.itemRequest], (ctx) => {
        return [Events.itemRequest.make({ item: ctx.self.item, to: ctx.self.to })]
    })
    .finish()
export const s2 = warehouse.designState('s2')
    .withPayload<ItemDeliveryPayload>()
    .command(EngineInstallationProtocol.cmdDeliver, [Events.itemDelivery], (ctx) => [Events.itemDelivery.make({item: ctx.self.item, to: ctx.self.to})])
    .finish()
export const s3 = warehouse.designEmpty('s3').finish()
export const s4 = warehouse.designEmpty('s4').finish()

s0.react([Events.requestEngine], s1, (_, event) => { return s1.make({ item: event.payload.item, to: event.payload.to }) })
s1.react([Events.itemRequest], s2, (_, event) => { return s2.make({ item: event.payload.item, to: event.payload.to }) })
s2.react([Events.itemDelivery], s3, (_) => { return s3.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([EngineInstallationProtocol.protocol], EngineInstallationProtocol.subscriptions, EngineInstallationProtocol.warehouseRole, warehouse.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))