import { MessageLogStore } from '@state/messageLogStore'
import { observer } from 'mobx-react-lite'
import { FormEvent, useState } from 'react'

interface InputBoxProps {
  messageLogStore: MessageLogStore
}

const InputBox = observer(({ messageLogStore }: InputBoxProps) => {
  const [userInput, setUserInput] = useState('')
  const promptStyle =
    userInput.length != 0 ? 'text-primary-text' : 'text-secondary-text'

  const handleOnSubmit = (e: FormEvent) => {
    e.preventDefault()

    if (userInput.length == 0) {
      return
    }

    messageLogStore.appendMessage({
      body: userInput,
      authorRef: 0,
      userRef: 0,
      timestamp: new Date(),
    })

    setUserInput('')
  }

  return (
    <form
      className="flex gap-2 rounded border-none bg-primary-color px-2 py-2"
      onSubmit={handleOnSubmit}
    >
      <button className={promptStyle}>&gt;</button>
      <input
        className="text-primary-text placeholder-secondary-text w-full bg-transparent focus:outline-none"
        type="text"
        placeholder="Enter message..."
        value={userInput}
        onChange={(e) => setUserInput(e.target.value)}
      />
    </form>
  )
})

export default InputBox
