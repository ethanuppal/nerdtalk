import MessageLogDisplay from '@components/MessageLogDisplay'
import InputBox from '@components/InputBox'
import messageLogStore from '@state/messageLogStore'

export default function PrimaryChatDisplay() {
  return (
    <div className="flex h-full flex-col gap-2">
      <MessageLogDisplay messageLogStore={messageLogStore} />

      <InputBox messageLogStore={messageLogStore} />
    </div>
  )
}
