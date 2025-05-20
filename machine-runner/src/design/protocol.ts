/* eslint-disable @typescript-eslint/no-explicit-any */
import { Tag, Tags } from '@actyx/sdk'
import { StateMechanism, MachineProtocol, ReactionMap, StateFactory, CommandDefinerMap } from './state.js'
import { Contained, MachineEvent } from './event.js'
import { Subscriptions, InterfacingSwarms, projectionInformation, ProjectionInfo } from '@actyx/machine-check'
import chalk = require('chalk');
import * as readline from 'readline';

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
  adaptMachine: <
    MachineName extends string,
    StateName extends string,
    StatePayload,
    Commands extends CommandDefinerMap<any, any, Contained.ContainedEvent<MachineEvent.Any>[]>,
  >(
    machineName: MachineName,
    role: MachineName,
    protocols: InterfacingSwarms,
    subscriptions: Subscriptions,
    mOldInitial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
    verbose?: boolean
  ) => MachineResult<[AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>, StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, StateName, StatePayload, Commands>]>
  adaptMachineNew: <
    MachineName extends string,
    StateName extends string,
    StatePayload,
    Commands extends CommandDefinerMap<any, any, Contained.ContainedEvent<MachineEvent.Any>[]>,
  >(
    machineName: MachineName,
    role: MachineName,
    protocols: InterfacingSwarms,
    subscriptions: Subscriptions,
    mOldInitial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
    verbose?: boolean
  ) => MachineResult<[AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>, StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, StateName, StatePayload, Commands>]>
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
      adaptMachine: (machineName, role, protocols, subscriptions, mOldInitial, verbose?) => {
        const projectionInfo = projectionInformation(protocols, subscriptions, role, true)
        if (projectionInfo.type == 'ERROR') {
          return {data: undefined, ... projectionInfo}
        }
        return ProjMachine.adaptMachine(ImplMachine.makeAdapted(swarmName, machineName, eventFactories, projectionInfo.data), eventFactories, mOldInitial, verbose)
      },
      adaptMachineNew: (machineName, role, protocols, subscriptions, mOldInitial, verbose?) => {
        const projectionInfo = projectionInformation(protocols, subscriptions, role, true)
        if (projectionInfo.type == 'ERROR') {
          return {data: undefined, ... projectionInfo}
        }
        return ProjMachine.adaptMachineNew(ImplMachine.makeAdapted(swarmName, machineName, eventFactories, projectionInfo.data), eventFactories, mOldInitial, verbose)
      }
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

interface AdaptedMachine<
  SwarmProtocolName extends string,
  MachineName extends string,
  MachineEventFactories extends MachineEvent.Factory.Any
> extends Machine<SwarmProtocolName, MachineName, MachineEventFactories>{
  readonly projectionInfo: ProjectionInfo
}

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

  /**
   * Create a machine protocol with a specific name, event factories,
   * a function mapping event types to sets of events types
   * and set of 'special event types' used for branch tracking.
   * @param machineName - name of the machine protocol.
   * @param registeredEventFactories - tuple of MachineEventFactories.
   * @see MachineEvent.design to get started on creating MachineEventFactories
   * for the registeredEventFactories parameter.
   * @example
   * const hangarBay = Machine.make("hangarBay")
   */
    export const makeAdapted = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    swarmName: SwarmProtocolName,
    machineName: MachineName,
    registeredEventFactories: MachineEventFactories[],
    projectionInfo: ProjectionInfo
  ): AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories> => {
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
      projectionInfo
    }
  }
}

export type ProjectionType = {
  initial: string
  transitions: {
    source: string
    target: string
    label: { tag: 'Execute'; cmd: string; logType: string[] } | { tag: 'Input'; eventType: string }
  }[]
}

export interface MachineAnalysisResource extends ProjectionType {
  subscriptions: string[]
}

export type MachineResult<T> = { type: 'OK'; data: T } | { type: 'ERROR'; errors: string[]; data: undefined }

//export type ProjectionInfo = { projection: ProjectionType, branches: Record<string, Set<string>>, specialEventTypes: Set<string> }

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

  export type funMap = {
    commands: Map<string, (...args: any[]) => any>,
    reactions: Map<string, (ctx: any, e: any) => any>
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
    projStatesToStatePayload: Map<string, (...args: any[]) => any>
  ): [Set<string>, ((...args: any[]) => any) | undefined] {
    function inner(
      s: string,
      initial: string,
      incomingEdgesMap: Map<string, Transition[]>,
      projStatesToStatePayload: Map<string, (...args: any[]) => any>,
      visited: Set<string>
    ): [Set<string>, Set<((...args: any[]) => any)>] {
      if (projStatesToStatePayload.has(s)) {
        return [new Set([s]), new Set([projStatesToStatePayload.get(s)!])]
      } else if (s === initial) {
        return [new Set(), new Set()]
      }

      const states: Set<string> = new Set()
      const fs: Set<((...args: any[]) => any)> = new Set()
      for (var t of incomingEdgesMap.get(s)!) {
        if (!visited.has(t.source)) {
          visited.add(t.source)
          const [preStates, preFs] = inner(t.source, initial, incomingEdgesMap, projStatesToStatePayload, visited)
          if (preStates.size != 0) {
            preStates.forEach(states.add, states)
            states.add(s)
            preFs.forEach(fs.add, fs)
          }
        }

      }

      return [states, fs]
    }

    const [states, fs] = inner(s, initial, incomingEdgesMap, projStatesToStatePayload, new Set())

    return [states, fs[Symbol.iterator]().next().value]
  }

  export type SucceedingNonBranchingJoining = Record<string, Set<string>>;
  export type ProjectionAndSucceedingMap = {
    projection: ProjectionType,
    branches: SucceedingNonBranchingJoining,
    specialEventTypes: Set<string>,
  }

  type ReactionLabel = {
    source: string;
    target: string;
    label: {
      tag: "Input";
      eventType: string;
    };
  }

  type CommandLabel = {
      source: string;
      target: string;
      label: {
        tag: "Execute";
        cmd: string;
        logType: string[];
      };
  }

  const printEvent = (e: any) => {
    const {lbj, ...toPrint} = e.payload
    console.log(chalk.bgBlack.blue`    ${e.payload.type}? ⬅ ${JSON.stringify(toPrint, null, 0)}`)
  }
  const printState = (machineName: string, stateName: string, statePayload: any) => {
    console.log(chalk.bgBlack.white.bold`${machineName} - State: ${stateName}. Payload: ${statePayload ? JSON.stringify(statePayload, null, 0) : "{}"}`)
  }
  const commandEnabledStrings = (labels: CommandLabel[] | undefined): string[] => labels ? labels.map(l => l.label.logType[0]) : []
  const printEnabledCmds = (labels: string[]) => {
    labels.forEach((transition) => {
      console.log(chalk.bgBlack.red.dim`    ${transition}!`);
    })
  }
  const printInfoOnTransition = (machineName: string, e: any, stateName: string, statePayload: any, labels: CommandLabel[] | undefined) => {
    printEvent(e);
    printState(machineName, stateName, statePayload);
    printEnabledCmds(commandEnabledStrings(labels));
  }

  const printEventEmission = (label: string, payload: string) => {
    readline.moveCursor(process.stdout, 0, -2);
    readline.clearScreenDown(process.stdout);
    console.log(chalk.bgBlack.green.bold`    ${label} ➡ ${payload}`);
  }

  export const adaptMachine = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
    StateName extends string,
    StatePayload,
    Commands extends CommandDefinerMap<any, any, Contained.ContainedEvent<MachineEvent.Any>[]>,
  >(
    mNew: AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>,
    events: readonly MachineEvent.Factory<any, Record<never, never>>[],
    mOldInitial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
    verbose?: boolean,
  ): MachineResult<[AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>, StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, StateName, StatePayload, Commands>]> => {
    var projStatesToStates: Map<string, any> = new Map()
    var projStatesToExec: Map<string, CommandLabel[]> = new Map()
    var projStatesToInput: Map<string, ReactionLabel[]> = new Map()
    // map event type string to Event
    const eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, Record<never, never>>> =
      new Map<string, MachineEvent.Factory<any, Record<never, never>>>(events.map(e => [e.type, e]))
    var projStatesToStatePayload: Map<string, (ctx: any, e: any) => any> = new Map()
    const proj = mNew.projectionInfo.projection
    var incomingMap = incomingEdgesOfStatesMap(proj)
    var markedStates: Set<string> = new Set()
    var fMap2: funMap = { commands: new Map(), reactions: new Map() }
    var allProjStates: Set<string> = new Set()

    mOldInitial.mechanism.protocol.reactionMap.getAll().forEach((reactionMapPerMechanism: any, stateMechanism: any) => {
      reactionMapPerMechanism.forEach((eventTypeEntry: any, eventType: any) => {
        fMap2.reactions.set(eventType, eventTypeEntry.handler)
      });
    });
    for (const factory of mOldInitial.mechanism.protocol.states.allFactories) {
      for (let [cmd, cmdDef] of Object.entries(factory.mechanism.commandDefinitions)) {
        fMap2.commands.set(cmd, cmdDef)
      };
    }
    proj.transitions.forEach((transition) => {
      if (transition.label.tag === 'Execute') {
        if (!projStatesToExec.has(transition.source)) {
          projStatesToExec.set(transition.source, new Array())
        }
        projStatesToExec.get(transition.source)?.push(transition as CommandLabel)

      } else if (transition.label.tag === 'Input') {
        if (!projStatesToInput.has(transition.source)) {
          projStatesToInput.set(transition.source, new Array())
        }
        projStatesToInput.get(transition.source)?.push(transition as ReactionLabel)

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

    const transitionToTriple = (transition: CommandLabel) => {
      if (verbose) {
        const f = fMap2.commands.get(transition.label.cmd)!
        const ff = (...args: any[]) => {
          const payload = f(...args);
          printEventEmission(`${transition.label.logType[0]}!`, `${JSON.stringify(payload[0], null, 0)}`)
          return payload;
        }
        return [transition.label.cmd, transition.label.logType.map((et: string) => eventTypeStringToEvent.get(et)), ff]
      }
      return [transition.label.cmd, transition.label.logType.map((et: string) => eventTypeStringToEvent.get(et)), fMap2.commands.get(transition.label.cmd)!]
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
      // works because non-zero numbers are truthy. design all states as carrying payload.
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
        const target = transition.target
        // a reaction like (s, e) => s3.make() does not create s3, it just creates the state payload. So calling it should be fine?
        if (verbose) {
          var f =
            fMap2.reactions.has(eventType)
              ? (ctx: any, e: any) => {
                const statePayload = fMap2.reactions.get(eventType)!(ctx, e); printInfoOnTransition(mNew.machineName, e, target, statePayload, projStatesToExec.get(target)); return statePayload }
              : (markedStates.has(target)
                ? (ctx: any, e: any) => { const statePayload = projStatesToStates.get(target).make(ctx.self); printInfoOnTransition(mNew.machineName, e, target, statePayload, projStatesToExec.get(target)); return statePayload } // propagate state payload
                : (ctx: any, e: any) => { printInfoOnTransition(mNew.machineName, e, target, undefined, projStatesToExec.get(target)); return projStatesToStates.get(target).make({}) })
        } else {
          var f =
            fMap2.reactions.has(eventType)
              ? fMap2.reactions.get(eventType)!
              : (markedStates.has(target)
                ? (ctx: any, e: any) => projStatesToStates.get(target).make(ctx.self) // propagate state payload
                : (ctx: any, e: any) => projStatesToStates.get(target).make({}))
        }

        projStatesToStates.get(transition.source).react([event], projStatesToStates.get(target), f)
      }
    }

    var initial = projStatesToStates.get(proj.initial)
    return {type: 'OK', data: [mNew, initial]}
  }

  // consider doing this sort of thing in machine-check. Seems ugly.
  function getStateNameAndID(stateName: string): string[] {
    const re = /(?<name>.*[^§])§(?<id>[\S\s]*)/;
    var groups = re.exec(stateName)?.groups;
      if (groups === undefined) {
        return ["", ""]
      } else {
        return [groups.name, groups.id]
      }
  }

  type ProjectionStateInfo = {
    projStateName: string,
    originalMStateName: string,
    reactionLabels: ReactionLabel[],
    commandLabels: CommandLabel[]
  }

  export const adaptMachineNew = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
    StateName extends string,
    StatePayload,
    Commands extends CommandDefinerMap<any, any, Contained.ContainedEvent<MachineEvent.Any>[]>,
  >(
    mNew: AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>,
    events: readonly MachineEvent.Factory<any, Record<never, never>>[],
    mOldInitial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
    verbose?: boolean,
  ): MachineResult<[AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>, StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, StateName, StatePayload, Commands>]> => {
    // information about projection states, such as their labels incoming and outgoing and what state in old machine they may correspond to
    const projStateInfoMap: Map<string, ProjectionStateInfo> = new Map()

    // map projection states to states in machine under constructions
    //any> = new Map()// string, any, Record<never, never>
    const projStateToMachineState: Map<string, StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, string, any, Record<never, never>>> = new Map()

    // we assume single event type commands. map command names to event types as strings
    const cmdToEventTypeString: Map<string, string> = new Map()

    // replace synthetic delimiter '§' with this string when creating machine states
    const replaceString = "_";
    //const replaceString = MachineAnalysisResource.SyntheticDelimiter;
    const makeOkMachineName = (oldName: String) => oldName.split(MachineAnalysisResource.SyntheticDelimiter).join(replaceString);

    // map state names in old machine to a map from command names to event type and command code
    const mOldStateToCommands: Map<string, Map<string, [string, any]>> = new Map()

    // map state names in old machine to a map from event type strings to reaction handler code
    const mOldStateToReactions: Map<string, Map<string, any>> = new Map()

    // map event type string to Event
    const eventTypeStringToEvent: Map<string, MachineEvent.Factory<any, Record<never, never>>> =
      new Map<string, MachineEvent.Factory<any, Record<never, never>>>(events.map(e => [e.type, e]))

      for (let t of mNew.projectionInfo.projection.transitions) {
        const sourceOriginalName = mNew.projectionInfo.projToMachineStates[t.source][0]
        const targetOriginalName = mNew.projectionInfo.projToMachineStates[t.target][0]

        if (!projStateInfoMap.has(t.source)) {
          projStateInfoMap.set(t.source, {projStateName: t.source, originalMStateName: sourceOriginalName, reactionLabels: [], commandLabels: []})
        }
        if (!projStateInfoMap.has(t.target)) {
          projStateInfoMap.set(t.target, {projStateName: t.target, originalMStateName: targetOriginalName, reactionLabels: [], commandLabels: []})
        }
        if (t.label.tag === 'Execute') {
          projStateInfoMap.get(t.source)?.commandLabels.push(t as CommandLabel)
        } else if (t.label.tag === 'Input') {
          projStateInfoMap.get(t.source)?.reactionLabels.push(t as ReactionLabel)
        }
        if (t.label.tag === 'Execute' && !cmdToEventTypeString.has(t.label.cmd)) {
          cmdToEventTypeString.set(t.label.cmd, t.label.logType[0])
        }
    }
    //console.log(projStateInfoMap)
    mOldInitial.mechanism.protocol.reactionMap.getAll().forEach((reactionMapPerMechanism: any, stateMechanism: any) => {
      const mStateName = stateMechanism.name;
      if (!mOldStateToReactions.has(mStateName)) {
        mOldStateToReactions.set(mStateName, new Map())
      }
      reactionMapPerMechanism.forEach((eventTypeEntry: any, eventType: any) => {
        mOldStateToReactions.get(mStateName)?.set(eventType, eventTypeEntry.handler)
      });
    });
    //console.log(mOldStateToReactions);
    for (const factory of mOldInitial.mechanism.protocol.states.allFactories) {
      const mStateName = factory.mechanism.name;
      for (let [cmd, cmdDef] of Object.entries(factory.mechanism.commandDefinitions)) {
        let eventTypeString = cmdToEventTypeString.get(cmd)!
        if (!mOldStateToCommands.has(mStateName)) {
          mOldStateToCommands.set(mStateName, new Map())
        }
        mOldStateToCommands.get(mStateName)?.set(cmd, [eventTypeString, cmdDef])
      };
    }
    //console.log(mOldStateToCommands)

    // add all states and self loops to machine
    projStateInfoMap.forEach((value: ProjectionStateInfo, key: string) => {
      if (value.commandLabels.length > 0) {
        let cmdTriples = new Array()
        for (const cLabel of value.commandLabels) {
          let cmdName = cLabel.label.cmd
          let eventTypes = cLabel.label.logType.map((et: string) => eventTypeStringToEvent.get(et))
          let code = mOldStateToCommands.get(value.originalMStateName)?.get(cmdName)![1]
          cmdTriples.push([cmdName, eventTypes, code])
        }
        //const cmdTriples1 = value.commandLabels.map((t: CommandLabel) => [t.label.cmd, t.label.logType.map((et: string) => eventTypeStringToEvent.get(et)), mOldStateToCommands.get(value.originalMStateName)?.get(t.label.cmd)![1]]).filter(triple => triple.length === 3)
        const thing = mNew.designState(value.projStateName).withPayload<any>().commandFromList(cmdTriples).finish()
        projStateToMachineState.set(value.projStateName, mNew.designState(value.projStateName).withPayload<any>().commandFromList(cmdTriples).finish())
      } else {
        projStateToMachineState.set(value.projStateName, mNew.designState(value.projStateName).withPayload<any>().finish())
      }
    });

    // add reactions
    projStateInfoMap.forEach((value: ProjectionStateInfo, key: string) => {
      for (const rLabel of value.reactionLabels) {
        const eventType = rLabel.label.eventType
        const event = eventTypeStringToEvent.get(eventType)!
        //const reactionHandler = mOldStateToReactions.get(value.originalMStateName)!.get(rLabel.label.eventType) ?? (s: any, _: any) => { return projStateToMachineState.get(rLabel.target).make(s.self) }
        //console.log(value.originalMStateName)
        //console.log(value.projStateName)
        //console.log(rLabel.source)
        //console.log(mOldStateToReactions.get(value.originalMStateName))
        const reactionHandler = mOldStateToReactions.get(value.originalMStateName)?.has(rLabel.label.eventType) ?
          mOldStateToReactions.get(value.originalMStateName)!.get(rLabel.label.eventType) : (s: any, _: any) => { return projStateToMachineState.get(rLabel.target).make(s.self) }
        projStateToMachineState.get(rLabel.source).react([event], projStateToMachineState.get(rLabel.target), reactionHandler)
      }
    })

    throw Error
    //return [mNew, projStateToMachineState.get(mNew.projectionInfo.projection.initial)]
  }
}
