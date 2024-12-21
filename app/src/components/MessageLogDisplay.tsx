import React, { useEffect } from 'react'
import TextBox from '@components/TextBox'
import { observer } from 'mobx-react-lite'
import { MessageLogStore } from '@state/messageLogStore'

interface MessageLogDisplayProps {
  messageLogStore: MessageLogStore
}

const MessageLogDisplay: React.FC<MessageLogDisplayProps> = observer(
  ({ messageLogStore }) => {
    useEffect(() => {
      messageLogStore.appendMessage({
        body: 'hello',
        sender: 'haadi',
        timestamp: 'blah',
      })
    }, [])

    return (
      <div className="flex flex-grow flex-col gap-1 overflow-scroll rounded border-none bg-secondary-color p-2 text-sm">
        {messageLogStore.runningLog.map((message) => (
          <TextBox timestamp={message.timestamp} messageBody={message.body} />
        ))}
      </div>
    )
  }
)

export default MessageLogDisplay
