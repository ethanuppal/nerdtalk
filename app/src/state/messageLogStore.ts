import { makeObservable, observable, action, computed } from 'mobx'
import { TextBoxProps, TextBoxType } from '@components/TextBox'

export interface Message {
  body: string
  timestamp: string
  author: string
  slotnum?: number
}

export class MessageLogStore {
  runningLog: Message[]

  constructor() {
    this.runningLog = []
    this.runningLog.push({
      body: 'hello',
      author: 'haadi',
      timestamp: 'blah',
    })
    this.runningLog.push({
      body: 'hello2',
      author: 'haadi',
      timestamp: 'blah',
    })    
    this.runningLog.push({
      body: 'hello2',
      author: 'haadi',
      timestamp: 'blah',
    })
    this.runningLog.push({
      body: 'hello',
      author: 'ethan',
      timestamp: 'blah',
    })

    makeObservable(this, {
      runningLog: observable,
      appendMessage: action,
      appendMessages: action,
      foldAuthors: computed,
    })
  }

  appendMessage(message: Message) {
    this.runningLog.push(message)
  }

  appendMessages(messages: Message[]) {
    this.runningLog = this.runningLog.concat(messages)
  }

  get foldAuthors() {
    let lastAuthor: string | null = null
    let messages: TextBoxProps[] = []

    for (const message of this.runningLog) {
      messages.push({
        type:
          lastAuthor === message.author
            ? TextBoxType.Trailing
            : TextBoxType.Authored,
        ...message,
      })
      lastAuthor = message.author
    }

    return messages
  }
}

const messageLogStore = new MessageLogStore()
export default messageLogStore
