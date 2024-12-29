import React from 'react'
import TextBox from '@components/TextBox'
import { observer } from 'mobx-react-lite'
import { MessageLogStore } from '@state/messageLogStore'
import userStore from '@state/authorStore'

interface MessageLogDisplayProps {
  messageLogStore: MessageLogStore
}

const MessageLogDisplay: React.FC<MessageLogDisplayProps> = observer(
  ({ messageLogStore }) => {
    return (
      <div className="flex flex-grow flex-col gap-1 overflow-scroll rounded border-none bg-primary-color py-2 text-sm">
        {messageLogStore.foldAuthors.map((message) => (
          <TextBox userStore={userStore} {...message} />
        ))}
      </div>
    )
  }
)

export default MessageLogDisplay
