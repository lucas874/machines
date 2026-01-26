import { SwarmProtocol } from '@actyx/machine-runner';
import { Events, transportOrderProtocol, assemblyLineProtocol, subscriptions } from './protocol'

export const AssemblyLine = SwarmProtocol.make('warehouse-factory', Events.allEvents)

export const AssemblyRobot = AssemblyLine.makeMachine('assemblyRobot')

export const InitialAssemblyRobot = AssemblyRobot.designEmpty('Initial')
  .finish()
export const Assemble = AssemblyRobot.designState('Assemble')
  .withPayload<{id: string}>()
  .command('assemble', [Events.product], (_ctx) =>
                         [{ productName: "product" }])
  .finish()
export const Done = AssemblyRobot.designEmpty('Done').finish()

// ingest the request from the `warehouse`
InitialAssemblyRobot.react([Events.ack], Assemble, (ctx, a) => ({
  id: a.payload.id
}))

// go to the final state
Assemble.react([Events.product], Done, (ctx, b) => {})

// Adapted machine.
export const [assemblyRobotAdapted, initialAssemblyAdapted] = AssemblyLine.adaptMachine('assemblyRobot', [transportOrderProtocol, assemblyLineProtocol], 1, subscriptions, [AssemblyRobot, InitialAssemblyRobot], true).data!
