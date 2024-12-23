import { format, isToday, isYesterday } from 'date-fns'

export enum TextBoxType {
  Authored,
  Trailing,
}

export interface TextBoxProps {
  type: TextBoxType
  body: string
  timestamp: Date
  author: string
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

export default function TextBox(props: TextBoxProps) {
  const { type, body, timestamp, author } = props

  if (type == TextBoxType.Authored)
    return (
      <div className="flex gap-2 px-2 text-white hover:bg-gray-600">
        <div className="px-1 grid place-content-center">
          <img
            src="/image.png"
            className={
              'w-9 rounded-full' + (author != 'haadi' ? ' invert' : '')
            }
          />
        </div>

        <div>
          <p>
            <span className="mr-1 font-bold">{author}</span>{' '}
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
      <div className="flex gap-2 px-2 text-secondary-color hover:bg-gray-600 hover:text-gray-400">
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
}
