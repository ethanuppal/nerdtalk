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
        author: 'haadi',
        timestamp: 'blah',
      })
    }, [])

    return (
      <div className="flex flex-grow flex-col gap-1 overflow-scroll rounded border-none bg-secondary-color text-sm">
        {messageLogStore.foldAuthors.map(message => (
          <TextBox {...message} />
        ))}
      </div>
    )
  }
)

export default MessageLogDisplay
