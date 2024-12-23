import { makeObservable, observable, action, computed } from 'mobx'
import { TextBoxInfo, TextBoxType } from '@components/TextBox'
import { AuthorRef } from '@state/authorStore'

export interface Message {
  body: string
  timestamp: Date
  authorRef: AuthorRef
  slotnum?: number
}

export class MessageLogStore {
  runningLog: Message[]

  constructor() {
    this.runningLog = []
    this.runningLog.push({
      body: 'hello',
      authorRef: 0,
      timestamp: new Date()
    })
    this.runningLog.push({
      body: 'hello2',
      authorRef: 0,
      timestamp: new Date(),
    })
    this.runningLog.push({
      body: 'hello2',
      authorRef: 1,
      timestamp: new Date(),
    })
    this.runningLog.push({
      body: 'hello',
      authorRef: 1,
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
    this.runningLog = this.runningLog.concat(messages)
  }

  get foldAuthors() {
    let lastAuthor: AuthorRef | null = null
    let messages: TextBoxInfo[] = []

    for (const message of this.runningLog) {
      messages.push({
        type:
          lastAuthor === message.authorRef
            ? TextBoxType.Trailing
            : TextBoxType.Authored,
        ...message,
      })
      lastAuthor = message.authorRef
    }

    return messages
  }
}

const messageLogStore = new MessageLogStore()
export default messageLogStore
