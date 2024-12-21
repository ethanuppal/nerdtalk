// import "./App.css";

import InputBox from '@components/InputBox'
import MessageLogDisplay from '@components/MessageLogDisplay'
import messageLogStore from '@state/messageLogStore'

function App() {
  return (
    <main className="flex h-screen flex-col gap-2 border-none p-2">
      <MessageLogDisplay messageLogStore={messageLogStore} />

      <InputBox messageLogStore={messageLogStore} />
    </main>
  )
}

export default App
