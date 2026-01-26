import { checkComposedProjection, checkComposedSwarmProtocol } from '@actyx/machine-check'
import { TransportRobot, InitialTransport } from './transport_robot'
import { Warehouse, InitialWarehouse } from './warehouse'
import { transportOrderProtocol, Events, assemblyLineProtocol, subscriptions } from './protocol'
import { AssemblyRobot, InitialAssemblyRobot } from './assembly_robot'
import { composeProtocols, InterfacingProtocols } from 'machine-core'

const transportRobotJSON =
  TransportRobot.createJSONForAnalysis(InitialTransport)
const warehouseJSON =
  Warehouse.createJSONForAnalysis(InitialWarehouse)
const subscriptionsTransportOrder = {
  transportRobot: transportRobotJSON.subscriptions,
  warehouse: warehouseJSON.subscriptions,
}
const assemblyRobotJSON =
  AssemblyRobot.createJSONForAnalysis(InitialAssemblyRobot)
const subscriptionsForAssemblyLine = {
  assemblyRobot: assemblyRobotJSON.subscriptions,
  warehouse: [Events.request.type, Events.ack.type],
}

// these should all print `{ type: 'OK' }`, otherwise there’s a mistake in the code
console.log(
  checkComposedSwarmProtocol([transportOrderProtocol], subscriptionsTransportOrder),
  checkComposedProjection([transportOrderProtocol], subscriptionsTransportOrder, 'transportRobot', transportRobotJSON),
  checkComposedProjection([transportOrderProtocol], subscriptionsTransportOrder, 'warehouse', warehouseJSON),
)

// these should all print `{ type: 'OK' }`
console.log(
  checkComposedSwarmProtocol([assemblyLineProtocol], subscriptionsForAssemblyLine),
  checkComposedProjection([assemblyLineProtocol], subscriptionsForAssemblyLine, 'assemblyRobot', assemblyRobotJSON)
)

// check that the subscription generated for the composition is indeed well-formed: this should print `{ type: 'OK' }`
console.log(
  checkComposedSwarmProtocol([transportOrderProtocol, assemblyLineProtocol], subscriptions)
)