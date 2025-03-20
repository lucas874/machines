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
  extendMachine: <MachineName extends string>(
    machineName: MachineName,
    proj: ProjMachine.ProjectionType,
    events: readonly MachineEvent.Factory<any, any>[],
    fMap: ProjMachine.funMap
  ) => [Machine<SwarmProtocolName, MachineName, MachineEventFactories>, any]
  extendMachine1: <MachineName extends string>(
    machineName: MachineName,
    proj: ProjMachine.ProjectionAndSucceedingMap,
    events: readonly MachineEvent.Factory<any, any>[],
    fMap: ProjMachine.funMap,
    mOldInitial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>
  ) => [Machine<SwarmProtocolName, MachineName, MachineEventFactories>, any]
  extendMachineBT: <MachineName extends string>(
    machineName: MachineName,
    proj: ProjMachine.ProjectionAndSucceedingMap,
    events: readonly MachineEvent.Factory<any, any>[],
    fMap: ProjMachine.funMap,
    mOrig: any,
  ) => [Machine<SwarmProtocolName, MachineName, MachineEventFactories>, any]
}

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
      extendMachine: (machineName, proj, events, fMap) => ProjMachine.extendMachine(ImplMachine.make(swarmName, machineName, eventFactories), proj, events, fMap),
      extendMachine1: (machineName, proj, events, fMap, mOldInitial) => ProjMachine.extendMachine1(ImplMachine.make(swarmName, machineName, eventFactories), proj, events, fMap, mOldInitial),
      extendMachineBT: (machineName, projectionInfo, events, fMap, mOrig) => ProjMachine.extendMachineBT(ImplMachine.make(swarmName, machineName, eventFactories), projectionInfo, events, fMap, mOrig)
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

  export type ReactionEntry = {
    genPayloadFun: (...args : any[]) => any
  }

  export type funMap = {
    commands: Map<any, any>,
    reactions: Map<any, ReactionEntry>
    initialPayloadType: ReactionEntry | undefined // awkward but used to indicate that initial state has some payload type
  }

  export type funMap2 = {
    commands: Map<string, (...args : any[]) => any>,
    reactions: Map<string, (...args : any[]) => any>
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

  // all the incoming edges of some state. including self loops
  function incomingEdgesOfStatesMap(proj: ProjectionType): Map<string, Transition[]> {
    const m: Map<string, Transition[]> = new Map()
    proj.transitions.forEach((transition) => {
      if (!m.has(transition.target)) {
        m.set(transition.target, [structuredClone(transition)])
      } else {
        m.get(transition.target)!.push(structuredClone(transition))
      }
    })
    if (!m.has(proj.initial)) {
      m.set(proj.initial, [])
    }

    return m
  }

  // States with same payload types as s and a function whose return type is this payload type
  // starting from some state move backwards in projection until we reach a state with a command
  // enabled or the initial state. If we reach a state with a reaction generating non void payload
  // add this state and all states between this state and s to the returned set of strings. f is the
  // reaction. should be the same for each base case since any intermediate states are due to
  // concurrency and subscribing to event types from other protocols.
  function payloadStates(
    s: string,
    initial: string,
    incomingEdgesMap: Map<string, Transition[]>,
    projStatesToStatePayload: Map<string, (...args : any[]) => any>
  ): [Set<string>, ((...args : any[]) => any) | undefined] {
    function inner(
      s: string,
      initial: string,
      incomingEdgesMap: Map<string, Transition[]>,
      projStatesToStatePayload: Map<string, (...args : any[]) => any>,
      visited: Set<string>
    ): [Set<string>, Set<((...args : any[]) => any)>] {
      if (projStatesToStatePayload.has(s)) {
        return [new Set([s]), new Set([projStatesToStatePayload.get(s)!])]
      } else if (s === initial) {
        return [new Set(), new Set()]
      }

      const states: Set<string> = new Set()
      const fs: Set<((...args : any[]) => any)> = new Set()
      for (var t of incomingEdgesMap.get(s)!) {
        if (!visited.has(t.source)) {
          visited.add(t.source)
          const [pre_states, pre_fs] = inner(t.source, initial, incomingEdgesMap, projStatesToStatePayload, visited)
          if (pre_states.size != 0) {
            pre_states.forEach(states.add, states)
            states.add(s)
            pre_fs.forEach(fs.add, fs)
          }
        }

      }

      return [states, fs]
    }

    const [states, fs] = inner(s, initial, incomingEdgesMap, projStatesToStatePayload, new Set())

    return [states, fs[Symbol.iterator]().next().value]
  }

  export const extendMachine = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    m: Machine<SwarmProtocolName, MachineName, MachineEventFactories>,
    proj: ProjectionType,
    events: readonly MachineEvent.Factory<any, Record<never, never>>[],
    fMap: funMap
  ): [Machine<SwarmProtocolName, MachineName, MachineEventFactories>, any]  => {
    var projStatesToStates: Map<string, any> = new Map()
    var projStatesToExec: Map<string, Transition[]> = new Map()
    var projStatesToInput: Map<string, Transition[]> = new Map()
    var eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, Record<never, never>>> = new Map()
    var projStatesToStatePayload: Map<string, (...args : any[]) => any> = new Map()
    var incomingMap = incomingEdgesOfStatesMap(proj)
    var markedStates: Set<string> = new Set()

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

        var e = transition.label.eventType
        if (fMap.reactions.has(e)) {
          projStatesToStatePayload.set(transition.target, fMap.reactions.get(e)!.genPayloadFun)
        }
      }

      if (transition.source === proj.initial
        && !projStatesToStatePayload.has(transition.source)
        && fMap.initialPayloadType !== undefined) {
          projStatesToStatePayload.set(transition.source, fMap.initialPayloadType!.genPayloadFun)
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
          cmdTriples.push([transition.label.cmd, es, f])
        }
      })

      const [statesWithSamePayloadType, f] = payloadStates(state, proj.initial, incomingMap, projStatesToStatePayload)

      if (f === undefined) {
        projStatesToStates.set(state, m.designEmpty(state).commandFromList(cmdTriples).finish())
      } else {
        projStatesToStates.set(state, m.designState(state).withPayload<ReturnType<typeof f>>().commandFromList(cmdTriples).finish())
        markedStates.add(state)
        for (const samePayloadState of statesWithSamePayloadType) {
          if (!projStatesToExec.has(samePayloadState) && !projStatesToStates.has(samePayloadState)) {
            projStatesToStates.set(samePayloadState, m.designState(samePayloadState).withPayload<ReturnType<typeof f>>().finish())
          }

          markedStates.add(samePayloadState)
        }
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
            fMap.reactions.has(e)
              ? (...args: any[]) => {const sPayload = fMap.reactions.get(e)!.genPayloadFun(...args); return projStatesToStates.get(transition.target).make(sPayload)}
              : (markedStates.has(transition.target) //(projStatesToExec.has(transition.target)
                  ? (s: any, _: any) => { return projStatesToStates.get(transition.target).make(s.self)}
                  : (_: any) => projStatesToStates.get(transition.target).make()
              )
          projStatesToStates.get(key).react([es], projStatesToStates.get(transition.target), f)
        }
      })
    })

    var initial = projStatesToStates.get(proj.initial)
    return [m, initial]
  }
  // https://medium.com/@alaneicker/how-to-process-json-data-with-recursion-dc530dd3db09
  export function loopThroughJSON(k: string, obj: any) {
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
        console.log(k + " " + key + ': ', obj[key]);
      }
    }
  }

  function isEmptyObject(value: unknown): boolean {
    return typeof(value) === "object" && value !== null && Object.keys(value).length === 0;
  }

  // TODO computeBranches: for each state in projection,
  // for each event type that has a transition in such state:
  // compute branch(e) --> set of states, follow each branch
  // add all event types encountered up to and including
  // first branching or joining event type.
  //function getEventTypesOnPaths(proj: )
  export type SucceedingNonBranchingJoining = Record<string, Set<string>>;
  export type ProjectionAndSucceedingMap = {
    projection: ProjectionType,
    succeeding_non_branching_joining: SucceedingNonBranchingJoining,
    branching_joining: Set<string>,
  }
  type LastJB = Map<string, string>
  type BTState<Payload> = {jbLast: LastJB, payload: Payload}
  type BTStateEmpty = BTState<undefined>

  export const extendMachineBT = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    m: Machine<SwarmProtocolName, MachineName, MachineEventFactories>,
    projectionInfo: ProjectionAndSucceedingMap,
    events: readonly MachineEvent.Factory<any, Record<never, never>>[],
    fMap: funMap,
    mOrig: any,
  ): [Machine<SwarmProtocolName, MachineName, MachineEventFactories>, any]  => {
    var projStatesToStates: Map<string, any> = new Map()
    var projStatesToExec: Map<string, Transition[]> = new Map()
    var projStatesToInput: Map<string, Transition[]> = new Map()
    var eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, Record<never, never>>> = new Map()
    var projStatesToStatePayload: Map<string, (...args : any[]) => any> = new Map()
    const proj = projectionInfo.projection
    const specialEvents = projectionInfo.branching_joining
    var incomingMap = incomingEdgesOfStatesMap(proj)
    var markedStates: Set<string> = new Set()
    /* for (var eee of events) {
      console.log("eee: ", loopThroughJSON("", eee))
    } */
    console.log("-------------")
    loopThroughJSON("", mOrig)
    console.log("-------------")
    console.log(mOrig.mechanism.protocol.reactionMap.getAll())
    console.log("-------------")
    //console.log(mOrig.mechanism.protocol.reactionMap.getAll().keys())
    //console.log("-------------")
    mOrig.mechanism.protocol.reactionMap.getAll().forEach((value: any, key: any) => {
      console.log("key: ", key)
      console.log("value: ", value)
      value.forEach((value_: any, key_: any) => {
        console.log("event type: ", key_)
        console.log("handler: ", value_.handler.toString())
        var tmp1: ReturnType<typeof value_.handler>
        var tmp2: Parameters<typeof value_.handler>
        //console.log("argument types of handler: ", typeof(tmp2))
        console.log("return type of handler: ", typeof(tmp1))
      });
      for (const p in key.commands) {
        console.log("command name: ", p)
        console.log("command: ", key.commands[p].toString())
      }
      console.log("-------------------");
  });
  for (var c of mOrig.mechanism.protocol.commands) {
    console.log("ccc: ", loopThroughJSON("", c))
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

        var e = transition.label.eventType
        if (fMap.reactions.has(e)) {
          projStatesToStatePayload.set(transition.target, fMap.reactions.get(e)!.genPayloadFun)
        }
      }

      if (transition.source === proj.initial
        && !projStatesToStatePayload.has(transition.source)
        && fMap.initialPayloadType !== undefined) {
          projStatesToStatePayload.set(transition.source, fMap.initialPayloadType!.genPayloadFun)
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

          //var f = fMap.commands.has(etypes[0]) ? fMap.commands.get(etypes[0]) : () => [{}]
          const payloadFun = fMap.commands.has(etypes[0]) ? fMap.commands.get(etypes[0]) : () => {}
          const f = (s: any, e: any) => {
            var payload = payloadFun({...s, self: s.self?.payload ?? s.self}, e);
            //var lbj = s.self?.lbj ?? null;
            console.log("HELLLOOOO ", s)
            var lbj = s.self.jbLast.get(etypes[0])
            console.log("HELLLOOOO ", lbj)
            console.log("HELLLOO ", es[0]?.makeBT(payload, lbj))
            console.log("HELLOO ", fMap.commands.has(etypes[0]))
            console.log("heeelo es[0]", es[0])
            console.log("typeof payload == {}?", typeof(payload) === typeof({}))
            console.log("typeof typeof: ", typeof(typeof(payload)))
            console.log("is empty object? ", isEmptyObject(payload))
            console.log("makeBT length: ", es[0]?.makeBT.length)
            if (es[0]?.makeBT.length == 1) {
              return [es[0]?.makeBT(payload, lbj)]
            }
            return [es[0]?.makeBT(payload, lbj)]
          }
          cmdTriples.push([transition.label.cmd, es, f])

        }
      })

      const [statesWithSamePayloadType, f] = payloadStates(state, proj.initial, incomingMap, projStatesToStatePayload)

      if (f === undefined) {
        projStatesToStates.set(state, m.designState(state).withPayload<BTStateEmpty>().commandFromList(cmdTriples).finish())
      } else {
        projStatesToStates.set(state, m.designState(state).withPayload<BTState<ReturnType<typeof f>>>().commandFromList(cmdTriples).finish())
        markedStates.add(state)
        for (const samePayloadState of statesWithSamePayloadType) {
          if (!projStatesToExec.has(samePayloadState) && !projStatesToStates.has(samePayloadState)) {
            projStatesToStates.set(samePayloadState, m.designState(samePayloadState).withPayload<ReturnType<typeof f>>().finish())
          }

          markedStates.add(samePayloadState)
        }
      }
    })

    projStatesToInput.forEach((value, key) => {
      if (!projStatesToStates.has(key)) {
        projStatesToStates.set(key, m.designState(key).withPayload<BTStateEmpty>().finish())
      }

      value.forEach((transition) => {
        if (transition.label.tag === 'Input') {
          if (!projStatesToStates.has(transition.target)) {
            projStatesToStates.set(transition.target, m.designState(transition.target).withPayload<BTStateEmpty>().finish())
          }
          var e = transition.label.eventType
          var es = eventTypeStringToEvent.get(e)

          const f = getReaction(e, fMap, projStatesToStates, markedStates, specialEvents, projectionInfo.succeeding_non_branching_joining, transition.target)
          projStatesToStates.get(key).react([es], projStatesToStates.get(transition.target), f)
        }
      })
    })

    var initial = projStatesToStates.get(proj.initial)
    return [m, initial]
  }

  function updateJBLast(t: string, eventId: string, jbLast: Map<string, string>, succeeding_non_branching_joining: Record<string, Set<string>>): Map<string, string> {
    const branchFromT = succeeding_non_branching_joining[t]
    var jbLastUpdated = structuredClone(jbLast)
    for (var et of branchFromT) {
      jbLastUpdated.set(et, eventId)
    }

    return jbLastUpdated
  }

  function getReaction(eventType: string, fMap: any, projStatesToStates: any, markedStates: any, specialEvents: Set<string>, succeeding_non_branching_joining: Record<string, Set<string>>, targetState: any) {
    if (fMap.reactions.has(eventType)) {
      if (specialEvents.has(eventType)) {
        return (s: any, e: any) => {
          const sPayload = fMap.reactions.get(eventType)!.genPayloadFun({...s, self: s.self?.payload ?? s.self}, e);
          console.log("ABOUT TO CRASH payload is: ", sPayload)
          console.log(typeof(sPayload))
          return projStatesToStates.get(targetState).make({jbLast: updateJBLast(e.payload.type, e.meta.eventId, s.self.jbLast, succeeding_non_branching_joining), payload: sPayload})
        }
      } else {
        return (s: any, e: any) => {
          const sPayload = fMap.reactions.get(eventType)!.genPayloadFun({...s, self: s.self?.payload ?? s.self}, e);
          console.log("ABOUT TO CRASH payload is: ", sPayload)
          console.log(typeof(sPayload))
          return projStatesToStates.get(targetState).make({jbLast: s.self.jbLast, payload: sPayload})
        }
      }
    } else if (markedStates.has(targetState)) {
      if (specialEvents.has(eventType)) {
        return (s: any, e: any) => {
          return projStatesToStates.get(targetState).make({jbLast: updateJBLast(e.payload.type, e.meta.eventId, s.self.jbLast, succeeding_non_branching_joining), payload: s.self.payload})
        }
      } else {
        return (s: any, e: any) => {
          return projStatesToStates.get(targetState).make(s.self)
        }
      }
    } else {
      if (specialEvents.has(eventType)) {
        return (s: any, e: any) => {
          return projStatesToStates.get(targetState).make({jbLast: updateJBLast(e.payload.type, e.meta.eventId, s.self.jbLast, succeeding_non_branching_joining), payload: undefined})
        }
      } else {
        return (s: any, e: any) => {
          return projStatesToStates.get(targetState).make({jbLast: s.self.jbLast, payload: undefined})
        }
      }
    }

  }

  export const extendMachine1 = <
  SwarmProtocolName extends string,
  MachineName extends string,
  MachineEventFactories extends MachineEvent.Factory.Any,
>(
  mNew: Machine<SwarmProtocolName, MachineName, MachineEventFactories>,
  projectionInfo: ProjectionAndSucceedingMap,
  events: readonly MachineEvent.Factory<any, Record<never, never>>[],
  fMap: funMap,
  mOldInitial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
): [Machine<SwarmProtocolName, MachineName, MachineEventFactories>, any]  => {
  var projStatesToStates: Map<string, any> = new Map()
  var projStatesToExec: Map<string, Transition[]> = new Map()
  var projStatesToInput: Map<string, Transition[]> = new Map()
  // map event type string to Event
  const eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, Record<never, never>>> =
    new Map<string, MachineEvent.Factory<any, Record<never, never>>>(events.map(e => [e.type, e]))
  var projStatesToStatePayload: Map<string, (...args : any[]) => any> = new Map()
  const proj = projectionInfo.projection
  const specialEvents = projectionInfo.branching_joining
  var incomingMap = incomingEdgesOfStatesMap(proj)
  var markedStates: Set<string> = new Set()
  var fMap2: funMap2 = {commands: new Map(), reactions: new Map()}
  var allProjStates: Set<string> = new Set()

  mOldInitial.mechanism.protocol.reactionMap.getAll().forEach((reactionMapPerMechanism: any, stateMechanism: any) => {
    reactionMapPerMechanism.forEach((eventTypeEntry: any, eventType: any) => {
      fMap2.reactions.set(eventType, eventTypeEntry.handler)
    });
    for (const command in stateMechanism.commands) {
      fMap2.commands.set(command, stateMechanism.commandDefinitions[command])
    }
  });

  proj.transitions.forEach((transition) => {
    if (transition.label.tag === 'Execute') {
      if (!projStatesToExec.has(transition.source)) {
        projStatesToExec.set(transition.source, new Array())
      }
      projStatesToExec.get(transition.source)?.push(transition)

    } else if (transition.label.tag === 'Input') {
      if (!projStatesToInput.has(transition.source)) {
        projStatesToInput.set(transition.source, new Array())
      }
      projStatesToInput.get(transition.source)?.push(transition)

      // add target to projStatesToInput as well. in case no outgoing transitions.
      if (!projStatesToInput.has(transition.target)) {
        projStatesToInput.set(transition.target, new Array())
      }

      var e = transition.label.eventType
      if (fMap2.reactions.has(e)) {
        projStatesToStatePayload.set(transition.target, fMap2.reactions.get(e)!)
      }
    }
    allProjStates.add(transition.source)
    allProjStates.add(transition.target)
  })

  const transitionToTriple = (transition: Transition) => {
    return transition.label.tag === 'Execute' ? [transition.label.cmd, transition.label.logType.map((et: string) => eventTypeStringToEvent.get(et)), fMap2.commands.get(transition.label.cmd)!] : []
  }

  // add all states from projection as states to machine
  // give them all payload type 'any'. Seems unsafe
  // but the way it was done previously was more complicated, while not being safer.
  for (var projState of allProjStates) {
    // commands from this state
    var cmdTriples: any[] = projStatesToExec.has(projState) ?
      projStatesToExec.get(projState)!.map(transitionToTriple).filter(triple => triple.length === 3) : []

    // get states to which we have to add a reaction propagating state payload
    const [statesWithSamePayloadType, _] = payloadStates(projState, proj.initial, incomingMap, projStatesToStatePayload)
    for (var samePayloadTypeState of statesWithSamePayloadType) {
      markedStates.add(samePayloadTypeState)
    }
    // works because non-zero numbers are truthy
    if (cmdTriples.length) {
      projStatesToStates.set(projState, mNew.designState(projState).withPayload<any>().commandFromList(cmdTriples).finish())
    } else {
      projStatesToStates.set(projState, mNew.designState(projState).withPayload<any>().finish())
    }
  }
  for (var transition of proj.transitions) {
    if (transition.label.tag === 'Input') {
      const eventType = transition.label.eventType
      const event = eventTypeStringToEvent.get(eventType)!
      // a reaction like (s, e) => s3.make() does not create s3, it just creates the state payload. So calling it should be fine?
      var f =
          fMap2.reactions.has(eventType)
            ? fMap2.reactions.get(eventType)!
            : (markedStates.has(transition.target)
                ? (s: any, _: any) => { return projStatesToStates.get(transition.target).make(s.self)} // propagate state payload
                : (_: any[]) => projStatesToStates.get(transition.target).make({}))
      projStatesToStates.get(transition.source).react([event], projStatesToStates.get(transition.target), f)

    }

  }

  var initial = projStatesToStates.get(proj.initial)
  return [mNew, initial]
}
}
