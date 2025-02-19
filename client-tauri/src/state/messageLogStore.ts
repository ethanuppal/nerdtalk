import { makeObservable, observable, action, computed } from 'mobx'
import { TextBoxInfo, TextBoxType } from '@components/TextBox'
import { UserRef } from '@state/authorStore'

export interface Message {
  body: string
  timestamp: Date
  userRef: UserRef
  slotnum?: number
}

export class MessageLogStore {
  runningLog: Message[]

  constructor() {
    this.runningLog = []
    this.runningLog.push({
      body: 'hello',
      userRef: 0,
      timestamp: new Date(),
    })
    this.runningLog.push({
      body: 'hello2',
      userRef: 0,
      timestamp: new Date(),
    })
    this.runningLog.push({
      body: 'hello2',
      userRef: 1,
      timestamp: new Date(),
    })
    this.runningLog.push({
      body: 'hello',
      userRef: 1,
      timestamp: new Date(),
    })

    makeObservable(this, {
      runningLog: observable,
      appendMessage: action,
      appendMessages: action,
      foldAuthors: computed,
    })
  }

  appendMessage(message: Message) {
    // TODO: Add filtering function
    if (message.body.length == 0) return

    this.runningLog.push(message)
  }

  appendMessages(messages: Message[]) {
    messages.forEach((message) => this.appendMessage(message))
  }

  get foldAuthors() {
    let lastUser: UserRef | null = null
    let messages: TextBoxInfo[] = []

    for (const message of this.runningLog) {
      messages.push({
        type:
          lastUser === message.userRef
            ? TextBoxType.Trailing
            : TextBoxType.Authored,
        ...message,
      })
      lastUser = message.userRef
    }

    return messages
  }
}

const messageLogStore = new MessageLogStore()
export default messageLogStore
