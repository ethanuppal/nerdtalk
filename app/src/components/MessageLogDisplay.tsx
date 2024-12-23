import React from 'react'
import TextBox from '@components/TextBox'
import { observer } from 'mobx-react-lite'
import { MessageLogStore } from '@state/messageLogStore'

interface MessageLogDisplayProps {
  messageLogStore: MessageLogStore
}

const MessageLogDisplay: React.FC<MessageLogDisplayProps> = observer(
  ({ messageLogStore }) => {
    return (
      <div className="flex flex-grow flex-col gap-1 overflow-scroll rounded border-none bg-secondary-color py-2 text-sm">
        {messageLogStore.foldAuthors.map((message) => (
          <TextBox {...message} />
        ))}
      </div>
    )
  }
)

export default MessageLogDisplay
