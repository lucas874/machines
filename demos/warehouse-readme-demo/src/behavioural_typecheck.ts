import { checkComposedProjection, checkComposedSwarmProtocol } from '@actyx/machine-check'
import { TransportRobot, Initial } from './transport_robot'
import { Warehouse, InitialWarehouse } from './warehouse'
import { transportOrderProtocol, Events, assemblyLineProtocol } from './protocol'
import { AssemblyRobot, AssemblyRobotInitial } from './assembly_robot'

const transportRobotJSON =
  TransportRobot.createJSONForAnalysis(Initial)
const warehouseJSON =
  Warehouse.createJSONForAnalysis(InitialWarehouse)
const subscriptionsTransportOrder = {
  transportRobot: transportRobotJSON.subscriptions,
  warehouse: warehouseJSON.subscriptions,
}
const assemblyRobotJSON =
  AssemblyRobot.createJSONForAnalysis(AssemblyRobotInitial)
const subscriptionsForAssemblyLine = {
  assemblyRobot: assemblyRobotJSON.subscriptions,
  warehouse: [Events.request.type, Events.ack.type],
}

// these should all print `{ type: 'OK' }`, otherwise there’s a mistake in
// the code (you would normally verify this using your favorite unit
// testing framework)
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

