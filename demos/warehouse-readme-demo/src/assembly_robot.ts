import { SwarmProtocol } from '@actyx/machine-runner';
import { Events, transportOrderProtocol, assemblyLineProtocol, subscriptions } from './protocol'

export const AssemblyProtocol = SwarmProtocol.make('TransportOrder', Events.allEvents)

export const AssemblyRobot = AssemblyProtocol.makeMachine('assemblyRobot')

export const AssemblyRobotInitial = AssemblyRobot.designEmpty('Initial')
  .finish()
export const Assemble = AssemblyRobot.designState('Assemble')
  .withPayload<{id: string}>()
  .command('assemble', [Events.product], (_ctx) =>
                         [{ productName: "product" }])
  .finish()
export const Done = AssemblyRobot.designEmpty('Done').finish()

// ingest the request from the `warehouse`
AssemblyRobotInitial.react([Events.ack], Assemble, (ctx, a) => ({
  id: a.payload.id
}))

// go to the final state
Assemble.react([Events.product], Done, (ctx, b) => {})

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
export const [assemblyRobotAdapted, initialAssemblyAdapted] = AssemblyProtocol.adaptMachine('assemblyRobot', [transportOrderProtocol, assemblyLineProtocol], 1, subscriptions, [AssemblyRobot, AssemblyRobotInitial], true).data!
