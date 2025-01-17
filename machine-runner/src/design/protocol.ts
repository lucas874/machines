/* eslint-disable @typescript-eslint/no-explicit-any */
import { MsgType, Tag, Tags } from '@actyx/sdk'
import { StateMechanism, MachineProtocol, ReactionMap, StateFactory } from './state.js'
import { MachineEvent } from './event.js'
import { DeepReadonly } from '../utils/type-utils.js'

/**
 * SwarmProtocol is the entry point of designing a swarm of MachineRunners. A
 * SwarmProtocol dictates MachineEvents used for communication and Actyx Tags
 * used as the channel to transport said Events. A SwarmProtocol provides a way
 * to design Machine protocols that abides the Events and Tags rules of the
 * SwarmProtocol.
 * @example
 * const protocol = SwarmProtocol.make("HangarBayExchange")
 * const machine = protocol.makeMachine("HangarBay")
 */
export type SwarmProtocol<
  SwarmProtocolName extends string,
  MachineEventFactories extends MachineEvent.Factory.Any,
  MachineEvents extends MachineEvent.Any = MachineEvent.Of<MachineEventFactories>,
> = {
  makeMachine: <MachineName extends string>(
    machineName: MachineName,
  ) => Machine<SwarmProtocolName, MachineName, MachineEventFactories>
  tagWithEntityId: (id: string) => Tags<MachineEvents>
  makeProjMachine: <MachineName extends string>(
    machineName: MachineName,
    proj: MachineAnalysisResource,
    events: readonly MachineEvent.Factory<any, any>[]
  ) => [Machine<any, any, any>, any]
  extendMachine: <MachineName extends string>(
    machineName: MachineName,
    proj: ProjMachine.ProjectionType,
    events: readonly MachineEvent.Factory<any, any>[],
    //mOriginal: [Machine<any, any, any>, any],
    fMap: ProjMachine.funMap
  ) => [Machine<any, any, any>, any]
}
//Machine<SwarmProtocolName, MachineName, MachineEventFactories>
/**
 * Utilities for SwarmProtocol
 * @see SwarmProtocol.make
 */
export namespace SwarmProtocol {
  /**
   * Construct a SwarmProtocol
   * @param swarmName - The name of the swarm protocol
   * @param tagString - the base tag used to mark the events passed to Actyx
   * @param registeredEventFactories - MachineEvent.Factories that are allowed
   * to be used for communications in the scope of this SwarmProtocol
   * @example
   * const HangarDoorTransitioning = MachineEvent
   *   .design("HangarDoorTransitioning")
   *   .withPayload<{ fractionOpen: number }>()
   * const HangarDoorClosed = MachineEvent
   *   .design("HangarDoorClosed")
   *   .withoutPayload()
   * const HangarDoorOpen = MachineEvent
   *   .design("HangarDoorOpen")
   *   .withoutPayload()
   *
   * // Creates a protocol
   * const HangarBay = SwarmProtocol.make(
   *   'HangarBay',
   *   [HangarDoorTransitioning, HangarDoorClosed, HangarDoorOpen]
   * )
   */
  export const make = <
    SwarmProtocolName extends string,
    InitialEventFactoriesTuple extends MachineEvent.Factory.ReadonlyNonZeroTuple,
  >(
    swarmName: SwarmProtocolName,
    registeredEventFactories: InitialEventFactoriesTuple,
  ): SwarmProtocol<SwarmProtocolName, MachineEvent.Factory.Reduce<InitialEventFactoriesTuple>> => {
    // Make a defensive copy to prevent side effects from external mutations
    const eventFactories = [
      ...registeredEventFactories,
    ] as MachineEvent.Factory.Reduce<InitialEventFactoriesTuple>[]
    type Factories = typeof eventFactories[0]
    const tag = Tag<MachineEvent.Of<Factories>>(swarmName)
    return {
      tagWithEntityId: (id) => tag.withId(id),
      makeMachine: (machineName) => ImplMachine.make(swarmName, machineName, eventFactories),
      makeProjMachine: (machineName, proj, events) => ProjMachine.machineFromProj(ImplMachine.make(swarmName, machineName, eventFactories), proj, events),
      //extendMachine: (machineName, proj, events, mOriginal, fMap) => ProjMachine.extendMachine(ImplMachine.make(swarmName, machineName, eventFactories), proj, events, mOriginal, fMap)
      extendMachine: (machineName, proj, events, fMap) => ProjMachine.extendMachine(ImplMachine.make(swarmName, machineName, eventFactories), proj, events, fMap)
    }
  }
}

/**
 * A machine is the entry point for designing machine states and transitions.
 * Its name should correspond to a role definition in a machine-check swarm
 * protocol. The resulting states are constrained to only be able to interact
 * with the events listed in the protocol design step. It accumulates
 * information on states and reactions. This information can be passed to
 * checkProjection to verify that the machine fits into a given swarm protocol.
 */
export type Machine<
  SwarmProtocolName extends string,
  MachineName extends string,
  MachineEventFactories extends MachineEvent.Factory.Any,
> = Readonly<{
  swarmName: SwarmProtocolName
  machineName: MachineName

  /**
   * Starts the design process for a state with a payload. Payload data will be
   * required when constructing this state.
   * @example
   * const HangarControlIncomingShip = machine
   *   .designState("HangarControlIncomingShip")
   *   .withPayload<{
   *     shipId: string,
   *   }>()
   *   .finish()
   */
  designState: <StateName extends string>(
    stateName: StateName,
  ) => DesignStateIntermediate<SwarmProtocolName, MachineName, MachineEventFactories, StateName>

  /**
   * Starts a design process for a state without a payload.
   * @example
   * const HangarControlIdle = machine
   *   .designEmpty("HangarControlIdle")
   *   .finish()
   */
  designEmpty: <StateName extends string>(
    stateName: StateName,
  ) => StateMechanism<
    SwarmProtocolName,
    MachineName,
    MachineEventFactories,
    StateName,
    void,
    Record<never, never>
  >

  createJSONForAnalysis: (
    initial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
  ) => MachineAnalysisResource
}>

type DesignStateIntermediate<
  SwarmProtocolName extends string,
  MachineName extends string,
  MachineEventFactories extends MachineEvent.Factory.Any,
  StateName extends string,
> = {
  /**
   * Declare payload type for a state.
   */
  withPayload: <StatePayload extends any>() => StateMechanism<
    SwarmProtocolName,
    MachineName,
    MachineEventFactories,
    StateName,
    StatePayload,
    Record<never, never>
  >
}

/**
 * A collection of type utilities around Machine.
 */
export namespace Machine {
  export type Any = Machine<any, any, any>

  /**
   * Extract the type of registered MachineEvent of a machine protocol in the
   * form of a union type.
   * @example
   * const E1 = MachineEvent.design("E1").withoutPayload();
   * const E2 = MachineEvent.design("E2").withoutPayload();
   * const E3 = MachineEvent.design("E3").withoutPayload();
   *
   * const protocol = SwarmProtocol.make("HangarBayExchange", [E1, E2, E3]);
   *
   * const machine = protocol.makeMachine("somename");
   *
   * type AllEvents = Machine.EventsOf<typeof machine>;
   * // Equivalent of:
   * // MachineEvent.Of<typeof E1> | MachineEvent.Of<typeof E2> | MachineEvent.Of<typeof E3>
   * // { "type": "E1" }           | { "type": "E2" }           | { "type": "E3" }
   */
  export type EventsOf<T extends Machine.Any> = T extends Machine<any, any, infer EventFactories>
    ? EventFactories
    : never
}

namespace ImplMachine {
  /**
   * Create a machine protocol with a specific name and event factories.
   * @param machineName - name of the machine protocol.
   * @param registeredEventFactories - tuple of MachineEventFactories.
   * @see MachineEvent.design to get started on creating MachineEventFactories
   * for the registeredEventFactories parameter.
   * @example
   * const hangarBay = Machine.make("hangarBay")
   */
  export const make = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    swarmName: SwarmProtocolName,
    machineName: MachineName,
    registeredEventFactories: MachineEventFactories[],
  ): Machine<SwarmProtocolName, MachineName, MachineEventFactories> => {
    type Self = Machine<SwarmProtocolName, MachineName, MachineEventFactories>
    type Protocol = MachineProtocol<SwarmProtocolName, MachineName, MachineEventFactories>

    const protocol: Protocol = {
      swarmName: swarmName,
      name: machineName,
      registeredEvents: registeredEventFactories,
      reactionMap: ReactionMap.make(),
      commands: [],
      states: {
        registeredNames: new Set(),
        allFactories: new Set(),
      },
    }

    const markStateNameAsUsed = (stateName: string) => {
      if (stateName.includes(MachineAnalysisResource.SyntheticDelimiter)) {
        throw new Error(
          `Name should not contain character '${MachineAnalysisResource.SyntheticDelimiter}'`,
        )
      }

      if (protocol.states.registeredNames.has(stateName)) {
        throw new Error(`State "${stateName}" already registered within this protocol`)
      }
      protocol.states.registeredNames.add(stateName)
    }

    const designState: Self['designState'] = (stateName) => {
      markStateNameAsUsed(stateName)
      return {
        withPayload: () => StateMechanism.make(protocol, stateName),
      }
    }

    const designEmpty: Self['designEmpty'] = (stateName) => {
      markStateNameAsUsed(stateName)
      return StateMechanism.make(protocol, stateName)
    }

    const createJSONForAnalysis: Self['createJSONForAnalysis'] = (initial) =>
      MachineAnalysisResource.fromMachineInternals(protocol, initial)

    return {
      swarmName,
      machineName,
      designState,
      designEmpty,
      createJSONForAnalysis,
    }
  }
}

export type MachineAnalysisResource = {
  initial: string
  subscriptions: string[]
  transitions: {
    source: string
    target: string
    label: { tag: 'Execute'; cmd: string; logType: string[] } | { tag: 'Input'; eventType: string }
  }[]
}

export namespace MachineAnalysisResource {
  export const SyntheticDelimiter = '§' as const

  export const syntheticEventName = (
    baseStateFactory: StateMechanism.Any | StateFactory.Any,
    modifyingEvents: Pick<MachineEvent.Factory.Any, 'type'>[],
  ) =>
    `${SyntheticDelimiter}${[
      ('mechanism' in baseStateFactory ? baseStateFactory.mechanism : baseStateFactory).name,
      ...modifyingEvents.map((f) => f.type),
    ].join(SyntheticDelimiter)}`

  export const fromMachineInternals = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    protocolInternals: MachineProtocol<SwarmProtocolName, MachineName, MachineEventFactories>,
    initial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
  ): MachineAnalysisResource => {
    if (!protocolInternals.states.allFactories.has(initial)) {
      throw new Error('Initial state supplied not found')
    }

    // Calculate transitions

    const reactionMapEntries = Array.from(protocolInternals.reactionMap.getAll().entries())

    const subscriptions: string[] = Array.from(
      new Set(
        reactionMapEntries.flatMap(([_, reactions]) =>
          Array.from(reactions.values()).flatMap((reaction): string[] =>
            reaction.eventChainTrigger.map((trigger) => trigger.type),
          ),
        ),
      ),
    )

    const transitionsFromReactions: MachineAnalysisResource['transitions'] =
      reactionMapEntries.reduce(
        (accumulated: MachineAnalysisResource['transitions'], [ofState, reactions]) => {
          for (const reaction of reactions.values()) {
            // This block converts a reaction into a chain of of transitions of states and synthetic states
            // Example:
            // A reacts to Events E1, E2, and E3 sequentially and transform into B
            // will result in these transitions
            // Source: A,       Input: E1, Target: A+E1
            // Source: A+E1,    Input: E2, Target: A+E1+E2
            // Source: A+E1+E2, Input: E3, Target: B
            const modifier: MachineEvent.Factory.Any[] = []
            for (const [index, trigger] of reaction.eventChainTrigger.entries()) {
              const source = index === 0 ? ofState.name : syntheticEventName(ofState, modifier)

              modifier.push(trigger)

              const target =
                index === reaction.eventChainTrigger.length - 1
                  ? reaction.next.mechanism.name
                  : syntheticEventName(ofState, modifier)

              accumulated.push({
                source: source,
                target: target,
                label: {
                  tag: 'Input',
                  eventType: trigger.type,
                },
              })
            }
          }

          return accumulated
        },
        [],
      )

    const transitionsFromCommands: MachineAnalysisResource['transitions'] =
      protocolInternals.commands.map((item): MachineAnalysisResource['transitions'][0] => ({
        source: item.ofState,
        target: item.ofState,
        label: {
          tag: 'Execute',
          cmd: item.commandName,
          logType: item.events,
        },
      }))

    const resource: MachineAnalysisResource = {
      initial: initial.mechanism.name,
      subscriptions,
      transitions: [...transitionsFromCommands, ...transitionsFromReactions],
    }

    return resource
  }
}

export namespace ProjMachine {
  type Transition = {
    source: string
    target: string
    label: { tag: 'Execute'; cmd: string; logType: string[] } | { tag: 'Input'; eventType: string }
  }
  // https://medium.com/@alaneicker/how-to-process-json-data-with-recursion-dc530dd3db09
  function loopThroughJSON(k: string, obj: any) {
    for (let key in obj) {
      if (typeof obj[key] === 'object') {
        if (Array.isArray(obj[key])) {
          // loop through array
          for (let i = 0; i < obj[key].length; i++) {
            loopThroughJSON(k + " " + key, obj[key][i]);
          }
        } else {
          // call function recursively for object
          loopThroughJSON(k + " " + key, obj[key]);
        }
      } else {
        // do something with value
        console.log(k + " " + key + ': ' + obj[key]);
      }
    }
  }
  //type ReplaceReturnType<T extends (...a: any) => any, TNewReturn> = (...a: Parameters<T>) => TNewReturn;

  export function machineFromProj(
    m: Machine<any, any, any>,
    proj: MachineAnalysisResource,
    events: readonly MachineEvent.Factory<any, MachineEvent.Any>[]
  ): [Machine<any, any, MachineEvent.Factory.Any>, any] {
    var projStatesToStates: Map<string, any> = new Map()
    var projStatesToExec: Map<string, Transition[]> = new Map()
    var projStatesToInput: Map<string, Transition[]> = new Map()
    var eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, MachineEvent.Any>> = new Map()

    proj.transitions.forEach((transition) => {
      if (transition.label.tag === 'Execute') {
        if (!projStatesToExec.has(transition.source)) {
          projStatesToExec.set(transition.source, new Array())
        }
        projStatesToExec.get(transition.source)?.push(transition)

        // map event type string to Event
        for (let eventType of transition.label.logType) {
          for (let event of events) {
              if (eventType === event.type) {
              eventTypeStringToEvent.set(eventType, event)
              break
            }
          }
        }

      } else if (transition.label.tag === 'Input') {
        if (!projStatesToInput.has(transition.source)) {
          projStatesToInput.set(transition.source, new Array())
        }
        projStatesToInput.get(transition.source)?.push(transition)

        // add target to projStatesToInput as well. in case no outgoing transitions.
        if (!projStatesToInput.has(transition.target)) {
          projStatesToInput.set(transition.target, new Array())
        }

        // map event type string to Event
        for (let event of events) {
          if (transition.label.eventType === event.type) {
            eventTypeStringToEvent.set(transition.label.eventType, event)
            break
          }
        }
      }
    })
    projStatesToExec.forEach((transitions, state) => {
      var test = new Array()
      // add self loops
      transitions.forEach((transition) => {
        if (transition.label.tag === 'Execute') {
          var eventTypes = transition.label.logType.map((et: string) => {
            return eventTypeStringToEvent.get(et)
          })
          test.push([transition.label.cmd, eventTypes, () => [{}]])
          //projStatesToStates.get(state).command(transition.label.cmd, eventTypes, () => [{}])
        }
      })

      projStatesToStates.set(state, m.designEmpty(state).commandFromList(test).finish())
    })

    projStatesToInput.forEach((value, key) => {
      if (!projStatesToStates.has(key)) {
        projStatesToStates.set(key, m.designEmpty(key).finish())
      }

      value.forEach((transition) => {
        if (transition.label.tag === 'Input') {
          if (!projStatesToStates.has(transition.target)) {
            projStatesToStates.set(transition.target, m.designEmpty(transition.target).finish())
          }

          projStatesToStates.get(key).react([eventTypeStringToEvent.get(transition.label.eventType)], projStatesToStates.get(transition.target), (_: any) => projStatesToStates.get(transition.target).make())
        }
      })
    })
    var initial = projStatesToStates.get(proj.initial)
    return [m, initial]
  }

  export type ReactionEntry = {
    identifiedByInput: boolean
    genPayloadFun: (...args : any[]) => any
  }

  export type funMap = {
    commands: Map<any, any>,
    reactions: Map<any, ReactionEntry>
  }

  export type ProjectionType = {
    initial: string;
    transitions: {
        source: string;
        target: string;
        label: {
            tag: "Execute";
            cmd: string;
            logType: string[];
        } | {
            tag: "Input";
            eventType: string;
        };
    }[];
  }

  export function extendMachineNotWorking(
    m: Machine<any, any, any>,
    proj: ProjectionType,
    events: readonly MachineEvent.Factory<any, Record<never, never>>[],
    mOriginal: [Machine<any, any, any>, any],
    fMap: funMap
  ): [Machine<any, any, MachineEvent.Factory.Any>, any] {
    var projStatesToStates: Map<string, any> = new Map()
    var projStatesToExec: Map<string, Transition[]> = new Map()
    var projStatesToInput: Map<string, Transition[]> = new Map()
    var eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, Record<never, never>>> = new Map()
    console.log("events arg: ")
    loopThroughJSON("", events)

    console.log("0: ")
    loopThroughJSON("", mOriginal[0])
    console.log("1: ")
    loopThroughJSON("", mOriginal[1])
    console.log("commands: ", mOriginal[1].mechanism.protocol.commands)
    var ccMap = new Map()
    for (var c in mOriginal[1].mechanism.protocol.commands) {
      console.log("cmd index is: ", c)
      console.log("cmd is: ", mOriginal[1].mechanism.protocol.commands[c])
      console.log("cmd event: ", mOriginal[1].mechanism.protocol.commands[c]['events'])
      console.log("works:? ", mOriginal[1].mechanism.commands[mOriginal[1].mechanism.protocol.commands[c]['commandName']])
      if (mOriginal[1].mechanism.commands[mOriginal[1].mechanism.protocol.commands[c]['commandName']] !== undefined) {
        var e = mOriginal[1].mechanism.protocol.commands[c]['events'][0]
        ccMap.set(e, mOriginal[1].mechanism.commands[mOriginal[1].mechanism.protocol.commands[c]['commandName']])
      }
      //ccMap.set(c['events'], mOriginal[1].mechanism.protocol.commands[c.commandName])
      console.log("works map? ", ccMap)
    }
    proj.transitions.forEach((transition) => {
      if (transition.label.tag === 'Execute') {
        if (!projStatesToExec.has(transition.source)) {
          projStatesToExec.set(transition.source, new Array())
        }
        projStatesToExec.get(transition.source)?.push(transition)

        // map event type string to Event
        for (let eventType of transition.label.logType) {
          for (let event of events) {
              if (eventType === event.type) {
              eventTypeStringToEvent.set(eventType, event)
              break
            }
          }
        }

      } else if (transition.label.tag === 'Input') {
        if (!projStatesToInput.has(transition.source)) {
          projStatesToInput.set(transition.source, new Array())
        }
        projStatesToInput.get(transition.source)?.push(transition)

        // add target to projStatesToInput as well. in case no outgoing transitions.
        if (!projStatesToInput.has(transition.target)) {
          projStatesToInput.set(transition.target, new Array())
        }

        // map event type string to Event
        for (let event of events) {
          if (transition.label.eventType === event.type) {
            eventTypeStringToEvent.set(transition.label.eventType, event)
            break
          }
        }
      }
    })
    projStatesToExec.forEach((transitions, state) => {
      var cmdTriples = new Array()
      // add self loops
      transitions.forEach((transition) => {
        if (transition.label.tag === 'Execute') {
          var es = transition.label.logType.map((et: string) => {
            return eventTypeStringToEvent.get(et)
          })
          var etypes = transition.label.logType.map((et: string) => {
            return eventTypeStringToEvent.get(et)?.type
          })
          var f = fMap.commands.has(etypes[0]) ? fMap.commands.get(etypes[0]) : () => [{}]
          console.log("ccmap ", ccMap.get(etypes[0]))
          //var f = ccMap.has(etypes[0]) ? ccMap.get(etypes[0]) : () => [{}]
          cmdTriples.push([transition.label.cmd, es, f])

        }
      })

      projStatesToStates.set(state, m.designEmpty(state).commandFromList(cmdTriples).finish())
    })

    projStatesToInput.forEach((value, key) => {
      if (!projStatesToStates.has(key)) {
        projStatesToStates.set(key, m.designEmpty(key).finish())
      }

      value.forEach((transition) => {
        if (transition.label.tag === 'Input') {
          if (!projStatesToStates.has(transition.target)) {
            projStatesToStates.set(transition.target, m.designEmpty(transition.target).finish())
          }
          var es = eventTypeStringToEvent.get(transition.label.eventType)
          //var f = fMap.reactions.has(eventType) ? fMap.reactions.get(eventType) : () => [{}]
          projStatesToStates.get(key).react([es], projStatesToStates.get(transition.target), (_: any) => projStatesToStates.get(transition.target).make())
        }
      })
    })

    var initial = projStatesToStates.get(proj.initial)
    return [m, initial]
  }

  export function extendMachine(
    m: Machine<any, any, any>,
    proj: ProjectionType,
    events: readonly MachineEvent.Factory<any, Record<never, never>>[],
    fMap: funMap
  ): [Machine<any, any, MachineEvent.Factory.Any>, any] {
    var projStatesToStates: Map<string, any> = new Map()
    var projStatesToExec: Map<string, Transition[]> = new Map()
    var projStatesToInput: Map<string, Transition[]> = new Map()
    var eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, Record<never, never>>> = new Map()
    var projStatesToStatePayload: Map<string, (s: any, e: any) => any> = new Map()
    proj.transitions.forEach((transition) => {
      if (transition.label.tag === 'Execute') {
        if (!projStatesToExec.has(transition.source)) {
          projStatesToExec.set(transition.source, new Array())
        }
        projStatesToExec.get(transition.source)?.push(transition)

        // map event type string to Event
        for (let eventType of transition.label.logType) {
          for (let event of events) {
              if (eventType === event.type) {
              eventTypeStringToEvent.set(eventType, event)
              break
            }
          }
        }
        var e = transition.label.logType[0]
        if (fMap.reactions.has(e) && !fMap.reactions.get(e)?.identifiedByInput) {
          projStatesToStatePayload.set(transition.source, fMap.reactions.get(e)!.genPayloadFun)
        }
      } else if (transition.label.tag === 'Input') {
        if (!projStatesToInput.has(transition.source)) {
          projStatesToInput.set(transition.source, new Array())
        }
        projStatesToInput.get(transition.source)?.push(transition)

        // add target to projStatesToInput as well. in case no outgoing transitions.
        if (!projStatesToInput.has(transition.target)) {
          projStatesToInput.set(transition.target, new Array())
        }

        // map event type string to Event
        for (let event of events) {
          if (transition.label.eventType === event.type) {
            eventTypeStringToEvent.set(transition.label.eventType, event)
            break
          }
        }

        var e = transition.label.eventType
        if (fMap.reactions.has(e) && fMap.reactions.get(e)?.identifiedByInput) {
          projStatesToStatePayload.set(transition.target, fMap.reactions.get(e)!.genPayloadFun)
        }
      }
    })
    projStatesToExec.forEach((transitions, state) => {
      var cmdTriples = new Array()
      // add self loops
      transitions.forEach((transition) => {
        if (transition.label.tag === 'Execute') {
          var es = transition.label.logType.map((et: string) => {
            return eventTypeStringToEvent.get(et)
          })
          var etypes = transition.label.logType.map((et: string) => {
            return eventTypeStringToEvent.get(et)?.type
          })
          var f = fMap.commands.has(etypes[0]) ? fMap.commands.get(etypes[0]) : () => [{}]
          //var f = ccMap.has(etypes[0]) ? ccMap.get(etypes[0]) : () => [{}]
          cmdTriples.push([transition.label.cmd, es, f])

        }
      })
      if (projStatesToStatePayload.has(state)) {
        var f = projStatesToStatePayload.get(state)!
        projStatesToStates.set(state, m.designState(state).withPayload<ReturnType<typeof f>>().commandFromList(cmdTriples).finish())
      } else {
        projStatesToStates.set(state, m.designEmpty(state).commandFromList(cmdTriples).finish())
      }

    })

    projStatesToInput.forEach((value, key) => {
      if (!projStatesToStates.has(key)) {
        projStatesToStates.set(key, m.designEmpty(key).finish())
      }

      value.forEach((transition) => {
        if (transition.label.tag === 'Input') {
          if (!projStatesToStates.has(transition.target)) {
            projStatesToStates.set(transition.target, m.designEmpty(transition.target).finish())
          }
          var e = transition.label.eventType
          var es = eventTypeStringToEvent.get(e)
          var f =
            fMap.reactions.has(e) && fMap.reactions.get(e)!.identifiedByInput
              ? (...args: any[]) => {const sPayload = fMap.reactions.get(e)!.genPayloadFun(...args); return projStatesToStates.get(transition.target).make(sPayload)}
              : (_: any) => projStatesToStates.get(transition.target).make()

          //var f = fMap.reactions.has(eventType) ? fMap.reactions.get(eventType) : () => [{}]
          //projStatesToStates.get(key).react([es], projStatesToStates.get(transition.target), (_: any) => projStatesToStates.get(transition.target).make())
          projStatesToStates.get(key).react([es], projStatesToStates.get(transition.target), f)
        }
      })
    })

    var initial = projStatesToStates.get(proj.initial)
    return [m, initial]
  }
}


/* if (transition.label.tag === 'Execute') {
          var eventTypes = transition.label.logType.map((et: string) => {
            return eventTypeStringToEvent.get(et)
          })
          projStatesToStates.get(key).command(transition.label.cmd, eventTypes, () => [{}])
        } else  */