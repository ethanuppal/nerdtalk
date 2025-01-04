class ClientBridge {
  private client: never

  constructor() {
    throw new Error('Not implemented yet')
  }

  /**
   * Uses Tauri binding to create new client
   */
  private initializeClient() {
    throw new Error('Not implemented yet')
  }

  /**
   * Attaches continuous listener to `client` awaiting new messages
   */
  private attachListener() {
    throw new Error('Not implemented yet')
  }

  /**
   * Forwards data over to client binding
   */
  private forwardData(data: never) {
    throw new Error('Not implemented yet')
  }
}
