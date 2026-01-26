import { SwarmProtocolType, InterfacingProtocols } from '../..'

export namespace WarehouseFactory {

  export const roleT = 'T'
  export const roleFL  = 'FL'
  export const roleD = 'D'
  export const roleR = 'R'

  export const cmdRequest = 'request'
  export const cmdGet = 'get'
  export const cmdDeliver = 'deliver'
  export const cmdClose = 'close'
  export const cmdBuild = 'build'

  export const eventTypePartReq = 'partReq'
  export const eventTypePartOk = 'partOk'
  export const eventTypePos = 'pos'
  export const eventTypeClosingTime = 'closingTime'
  export const eventTypeCar = 'car'

  export const warehouse: SwarmProtocolType = {
    initial: '0',
    transitions: [
      {source: '0', target: '1', label: {cmd: cmdRequest, role: roleT, logType: [eventTypePartReq]}},
      {source: '1', target: '2', label: {cmd: cmdGet, role: roleFL, logType: [eventTypePos]}},
      {source: '2', target: '0', label: {cmd: cmdDeliver, role: roleT, logType: [eventTypePartOk]}},
      {source: '0', target: '3', label: {cmd: cmdClose, role: roleD, logType: [eventTypeClosingTime]}},
    ]
  }

  export const factory: SwarmProtocolType = {
    initial: '0',
    transitions: [
      {source: '0', target: '1', label: { cmd: cmdRequest, role: roleT, logType: [eventTypePartReq]}},
      {source: '1', target: '2', label: { cmd: cmdDeliver, role: roleT, logType: [eventTypePartOk]}},
      {source: '2', target: '3', label: { cmd: cmdBuild, role: roleR, logType: [eventTypeCar]}},
    ]
  }
 
  export const protocols: InterfacingProtocols = [warehouse, factory]
}