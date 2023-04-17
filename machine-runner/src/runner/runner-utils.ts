import { ActyxEvent } from '@actyx/sdk'
import { StateRaw, StateFactory, StateMechanism } from '../design/state.js'
import { MachineEvent } from '../design/event.js'
import { PushEventResult } from './runner-internals.js'
import { EventMap } from 'typed-emitter'
import { StateOpaque } from './runner.js'

/**
 * Imported this way because it cannot be imported via normal import ... from
 * syntax https://github.com/andywer/typed-emitter/issues/39
 */
type TypedEventEmitter<Events extends EventMap> = import('typed-emitter').default<Events>

export type MachineEmitter<
  SwarmProtocolName extends string,
  MachineName extends string,
> = TypedEventEmitter<MachineEmitterEventMap<SwarmProtocolName, MachineName>>

export type MachineEmitterEventMap<SwarmProtocolName extends string, MachineName extends string> = {
  'audit.reset': (_: void) => unknown
  'audit.state': (_: {
    state: StateOpaque<SwarmProtocolName, MachineName>
    events: ActyxEvent<MachineEvent.Any>[]
  }) => unknown
  'audit.dropped': (_: { state: StateRaw.Any; event: ActyxEvent<MachineEvent.Any> }) => unknown
  'audit.error': (_: {
    state: StateRaw.Any
    events: ActyxEvent<MachineEvent.Any>[]
    error: unknown
  }) => unknown
  'debug.eventHandlingPrevState': (_: unknown) => unknown
  'debug.eventHandling': (_: {
    event: ActyxEvent<MachineEvent.Any>
    handlingReport: PushEventResult
    mechanism: StateMechanism.Any
    factory: StateFactory.Any
    nextState: unknown
  }) => unknown
  change: (_: StateOpaque<SwarmProtocolName, MachineName>) => unknown
  destroyed: (_: void) => unknown
  log: (_: string) => unknown
}