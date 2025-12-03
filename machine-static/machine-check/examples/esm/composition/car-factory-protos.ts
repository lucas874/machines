/* eslint-disable @typescript-eslint/no-namespace */
import { MachineEvent, SwarmProtocol } from '@actyx/machine-runner'

/*
 * Example from CoPLaWS slides by Florian Furbach
 */

export namespace Events {
  export const partID = MachineEvent.design('partID').withoutPayload()
  export const part = MachineEvent.design('part').withoutPayload()
  export const position = MachineEvent.design('position').withoutPayload()
  export const time = MachineEvent.design('time').withoutPayload()
  export const car = MachineEvent.design('car').withoutPayload()
  export const report = MachineEvent.design('report').withoutPayload()
  export const notOk = MachineEvent.design('notOk').withoutPayload()
  export const ok = MachineEvent.design('ok').withoutPayload()

  export const eventsWarehouse = [partID, part, position, time] as const
  export const eventsCarFactory = [partID, part, car] as const
  export const eventsQualityControl = [car, report, notOk, ok] as const
  export const eventsAll = [partID, part, position, time, car, report, notOk, ok] as const

}


export const Warehouse = SwarmProtocol.make('Warehouse', Events.eventsWarehouse)
export const CarFactory = SwarmProtocol.make('CarFactory', Events.eventsCarFactory)
export const QualityControl = SwarmProtocol.make('CarFactory', Events.eventsQualityControl)
export const Composition = SwarmProtocol.make('Composition', Events.eventsAll)