import { AuthorRef, AuthorStore } from '@state/authorStore'
import { format, isToday, isYesterday } from 'date-fns'
import { observer } from 'mobx-react-lite'

export enum TextBoxType {
  Authored,
  Trailing,
}

export interface TextBoxInfo {
  type: TextBoxType
  body: string
  timestamp: Date
  authorRef: AuthorRef,
}

export interface TextBoxProps extends TextBoxInfo {
  authorStore: AuthorStore
}

function formatMessageTrailingDate(date: Date): string {
  return format(date, 'h:mm aa')
}

function formatMessageAuthorDate(date: Date): string {
  let day_str

  if (isToday(date)) {
    day_str = 'Today'
  } else if (isYesterday(date)) {
    day_str = 'Yesterday'
  } else {
    day_str = format(date, 'EEEE')
  }

  return `${day_str} at ${formatMessageTrailingDate(date)}`
}

const TextBox = observer((props: TextBoxProps) => {
  const { type, body, timestamp, authorRef, authorStore } = props
  const author = authorStore.authors[authorRef]

  if (type == TextBoxType.Authored)
    return (
      <div className="flex gap-2 px-2 text-white hover:bg-gray-600">
        <div className="grid place-content-center px-1">
          <img src={author.avatarURL} className="w-9 rounded-full" />
        </div>

        <div>
          <p>
            <span className="mr-1 font-bold">{author.name}</span>{' '}
            <time className="text-xxs text-gray-400">
              {formatMessageAuthorDate(timestamp)}
            </time>{' '}
          </p>
          <p>{body}</p>
        </div>
      </div>
    )
  else
    return (
      <div className="flex gap-2 px-2 text-primary-color hover:bg-gray-600 hover:text-gray-400">
        <div className="mx-1 grid h-5 w-9 place-content-center">
          <div className="h-fit text-center">
            <time className="box-content block text-[0.5rem] leading-[0.5rem]">
              {formatMessageTrailingDate(timestamp)}
            </time>
          </div>
        </div>
        <p className="!text-white">{body}</p>
      </div>
    )
})

export default TextBox