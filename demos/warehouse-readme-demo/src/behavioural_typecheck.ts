import { checkComposedProjection, checkComposedSwarmProtocol } from '@actyx/machine-check'
import { TransportOrderForRobot, Initial } from './transport_robot'
import { TransportOrderForWarehouse, InitialWarehouse } from './warehouse'
import { warehouseProtocol } from './protocol'

const robotJSON =
  TransportOrderForRobot.createJSONForAnalysis(Initial)
const warehouseJSON =
  TransportOrderForWarehouse.createJSONForAnalysis(InitialWarehouse)
const subscriptions = {
  robot: robotJSON.subscriptions,
  warehouse: warehouseJSON.subscriptions,
}

// these should all print `{ type: 'OK' }`, otherwise there’s a mistake in
// the code (you would normally verify this using your favorite unit
// testing framework)
console.log(
  checkComposedSwarmProtocol([warehouseProtocol], subscriptions),
  checkComposedProjection([warehouseProtocol], subscriptions, 'robot', robotJSON),
  checkComposedProjection([warehouseProtocol], subscriptions, 'warehouse', warehouseJSON),
)