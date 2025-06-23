/* eslint-disable @typescript-eslint/no-explicit-any */
import { ActyxEvent, Tag, Tags } from '@actyx/sdk'
import { StateMechanism, MachineProtocol, ReactionMap, StateFactory, CommandDefinerMap, ReactionHandler } from './state.js'
import { Contained, MachineEvent } from './event.js'
import { Subscriptions, projectionInformation, ProjectionInfo, InterfacingProtocols } from '@actyx/machine-check'
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
    ProjectionName extends string,
    StateName extends string,
    StatePayload,
    Commands extends CommandDefinerMap<any, any, Contained.ContainedEvent<MachineEvent.Any>[]>,
  >(
    role: ProjectionName,
    protocols: InterfacingProtocols,
    k: number,
    subscriptions: Subscriptions,
    mOld: [Machine<SwarmProtocolName, MachineName, MachineEventFactories>, StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>],
    verbose?: boolean
  ) => MachineResult<[AdaptedMachine<SwarmProtocolName, ProjectionName, MachineEventFactories>, StateFactory<SwarmProtocolName, ProjectionName, MachineEventFactories, StateName, StatePayload, Commands>]>
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
      adaptMachine: (role, protocols, k, subscriptions, oldMachine, verbose?) => {
        const [mOld, mOldInitial] = oldMachine
        const projectionInfo = projectionInformation(protocols, subscriptions, role, mOld.createJSONForAnalysis(mOldInitial), k, true)
        if (projectionInfo.type == 'ERROR') {
          return {data: undefined, ... projectionInfo}
        }
        return MachineAdaptation.adaptMachine(ImplMachine.makeAdapted(swarmName, role, eventFactories, projectionInfo.data), eventFactories, mOldInitial, verbose)
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

export interface AdaptedMachine<
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

export namespace MachineAdaptation {
  // f is the
  // reaction. should be the same for each base case since any intermediate states are due to
  // concurrency and subscribing to event types from other protocols.

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

  const verboseCommandDef = (label: CommandLabel, commandDef: any) => {
    const verboseCommandDef = (...args: any[]) => {
      const payload = commandDef(...args);
      printEventEmission(`${label.label.logType[0]}!`, `${JSON.stringify(payload[0], null, 0)}`)
      return payload;
    }
    return verboseCommandDef
  }

  const verboseReaction = <Context>(
    reactionHandler: ReactionHandler<ActyxEvent<MachineEvent.Any>[], Context, unknown>,
    machineName: string,
    targetState: string,
    cmdsEnabledAtTarget: CommandLabel[],
  ): ReactionHandler<ActyxEvent<MachineEvent.Any>[], Context, unknown> => {
    return (ctx: Context, event: ActyxEvent<MachineEvent.Any>) => {
      const statePayload = reactionHandler(ctx, event);
      printInfoOnTransition(machineName, event, targetState, statePayload, cmdsEnabledAtTarget);
      return statePayload
    }
  }

  type ProjectionStateInfo = {
    projStateName: string,
    originalMStateName: string,
    reactionLabels: ReactionLabel[],
    commandLabels: CommandLabel[]
  }

  // Information about projection states, such as their labels incoming and outgoing and what state in some machine they may correspond to
  const projStateInfo = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    m: AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>
  ): Map<string, ProjectionStateInfo> => {
    const projStateInfoMap: Map<string, ProjectionStateInfo> = new Map()

    for (let t of m.projectionInfo.projection.transitions) {
        const sourceOriginalName = m.projectionInfo.projToMachineStates[t.source][0]
        const targetOriginalName = m.projectionInfo.projToMachineStates[t.target][0]

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
    }
    return projStateInfoMap
  }

  // We assume single event type commands. map command names to event types as strings
  const cmdNameToEventType = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    m: AdaptedMachine<SwarmProtocolName, MachineName, MachineEventFactories>
  ): Map<string, string> => {
    const cmdToEventTypeString: Map<string, string> = new Map()
    for (let t of m.projectionInfo.projection.transitions) {
        if (t.label.tag === 'Execute' && !cmdToEventTypeString.has(t.label.cmd)) {
          cmdToEventTypeString.set(t.label.cmd, t.label.logType[0])
        }
    }
    return cmdToEventTypeString
  }

  // Map state names in a machine to a map from event type strings to reaction handler code
  const mStateToReactions = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    mState: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
  ): Map<string, Map<string, any>> => {
    const mStateToReactionsMap: Map<string, Map<string, any>> = new Map()
    mState.mechanism.protocol.reactionMap.getAll().forEach((reactionMapPerMechanism: any, stateMechanism: any) => {
      const mStateName = stateMechanism.name;
      if (!mStateToReactionsMap.has(mStateName)) {
        mStateToReactionsMap.set(mStateName, new Map())
      }
      reactionMapPerMechanism.forEach((eventTypeEntry: any, eventType: any) => {
        mStateToReactionsMap.get(mStateName)?.set(eventType, eventTypeEntry.handler)
      });
    });

    return mStateToReactionsMap
  }

  // Map state names in a machine to a map from command names to event type and command code
  const mStateToCommands = <
    SwarmProtocolName extends string,
    MachineName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
  >(
    mState: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
    cmdToEventTypeString: Map<string, string>
  ): Map<string, Map<string, [string, any]>> => {
    const mStateToCommandsMap: Map<string, Map<string, [string, any]>> = new Map()
    for (const factory of mState.mechanism.protocol.states.allFactories) {
      const mStateName = factory.mechanism.name;
      for (let [cmd, cmdDef] of Object.entries(factory.mechanism.commandDefinitions)) {
        let eventTypeString = cmdToEventTypeString.get(cmd)!
        if (!mStateToCommandsMap.has(mStateName)) {
          mStateToCommandsMap.set(mStateName, new Map())
        }
        mStateToCommandsMap.get(mStateName)?.set(cmd, [eventTypeString, cmdDef])
      };
    }

    return mStateToCommandsMap
  }

  export const adaptMachine = <
    SwarmProtocolName extends string,
    MachineName extends string,
    ProjectionName extends string,
    MachineEventFactories extends MachineEvent.Factory.Any,
    StateName extends string,
    StatePayload,
    Commands extends CommandDefinerMap<any, any, Contained.ContainedEvent<MachineEvent.Any>[]>,
  >(
    mNew: AdaptedMachine<SwarmProtocolName, ProjectionName, MachineEventFactories>,
    events: readonly MachineEventFactories[],
    mOldInitial: StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, any, any, any>,
    verbose?: boolean,
  ): MachineResult<[AdaptedMachine<SwarmProtocolName, ProjectionName, MachineEventFactories>, StateFactory<SwarmProtocolName, ProjectionName, MachineEventFactories, StateName, StatePayload, Commands>]> => {
    // information about projection states, such as their labels incoming and outgoing and what state in old machine they may correspond to
    const projStateInfoMap: Map<string, ProjectionStateInfo> = projStateInfo(mNew)

    // map projection states to states in machine under constructions
    const projStateToMachineState: Map<string, StateFactory<SwarmProtocolName, ProjectionName, MachineEventFactories, StateName, StatePayload, Commands>> = new Map()

    // we assume single event type commands. map command names to event types as strings
    const cmdToEventTypeString: Map<string, string> = cmdNameToEventType(mNew)

    // map state names in old machine to a map from command names to event type and command code
    const mOldStateToCommands: Map<string, Map<string, [string, any]>> = mStateToCommands(mOldInitial, cmdToEventTypeString)

    // map state names in old machine to a map from event type strings to reaction handler code
    const mOldStateToReactions: Map<string, Map<string, any>> = mStateToReactions(mOldInitial)

    // map event type string to Event
    const eventTypeStringToEvent: Map<string, MachineEventFactories> =
      new Map<string, MachineEventFactories>(events.map(e => [e.type, e]))

    // add all states and self loops to machine
    projStateInfoMap.forEach((value: ProjectionStateInfo, key: string) => {
      if (value.commandLabels.length > 0) {
        let cmdTriples = new Array()
        for (const cLabel of value.commandLabels) {
          let cmdName = cLabel.label.cmd
          let eventTypes = cLabel.label.logType.map((et: string) => eventTypeStringToEvent.get(et))
          const commandDef = verbose ? verboseCommandDef(cLabel, mOldStateToCommands.get(value.originalMStateName)?.get(cmdName)![1]) : mOldStateToCommands.get(value.originalMStateName)?.get(cmdName)![1]
          cmdTriples.push([cmdName, eventTypes, commandDef])
        }

        const newState = mNew.designState(value.projStateName).withPayload<any>().commandFromList(cmdTriples).finish() as StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, StateName, StatePayload, Commands>
        projStateToMachineState.set(value.projStateName, newState)
      } else {
        const newState = mNew.designState(value.projStateName).withPayload<any>().finish() as StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, StateName, StatePayload, Commands>
        projStateToMachineState.set(value.projStateName, newState)
      }
    });

    // add reactions
    projStateInfoMap.forEach((value: ProjectionStateInfo, key: string) => {
      for (const rLabel of value.reactionLabels) {
        const eventType = rLabel.label.eventType
        const event = eventTypeStringToEvent.get(eventType)!

        const reactionHandler = mOldStateToReactions.get(value.originalMStateName)?.has(rLabel.label.eventType)
          ? (verbose ? verboseReaction(mOldStateToReactions.get(value.originalMStateName)!.get(rLabel.label.eventType), mNew.machineName, rLabel.target, projStateInfoMap.get(rLabel.target)!.commandLabels) : mOldStateToReactions.get(value.originalMStateName)!.get(rLabel.label.eventType))
          : (verbose ? verboseReaction((ctx: any, e: any) => { return projStateToMachineState.get(rLabel.target)!.make(ctx.self) }, mNew.machineName, rLabel.target, projStateInfoMap.get(rLabel.target)!.commandLabels) : (ctx: any, e: any) => { return projStateToMachineState.get(rLabel.target)!.make(ctx.self) })

        const targetState = projStateToMachineState.get(rLabel.target)! as StateFactory<SwarmProtocolName, MachineName, MachineEventFactories, string, unknown, any>
        projStateToMachineState.get(rLabel.source)!.react([event], targetState, reactionHandler)
      }
    })

    const initial = projStateToMachineState.get(mNew.projectionInfo.projection.initial)!
    return { type: 'OK', data: [mNew, initial] }
  }
}