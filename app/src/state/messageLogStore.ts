import { makeObservable, observable, action } from 'mobx'

export interface Message {
  body: string
  timestamp: string
  sender: string
  slotnum?: number
}

export class MessageLogStore {
  runningLog: Message[]

  constructor() {
    this.runningLog = []
    makeObservable(this, {
      runningLog: observable,
      appendMessage: action,
      appendMessages: action,
    })
  }

  appendMessage(message: Message) {
    this.runningLog.push(message)
  }

  appendMessages(messages: Message[]) {
    this.runningLog = this.runningLog.concat(messages)
  }
}

const messageLogStore = new MessageLogStore()
export default messageLogStore
