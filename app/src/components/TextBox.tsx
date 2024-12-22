export enum TextBoxType {
  Authored,
  Trailing,
}

export interface TextBoxProps {
  type: TextBoxType
  body: string
  timestamp: string
  author: string
}

export default function TextBox(props: TextBoxProps) {
  const { type, body, timestamp, author } = props

  return <div className="className=py-2 text-white hover:bg-gray-600">
    <p><span className="font-bold">{author}</span> <time className="text-gray-400 text-xs">{timestamp}</time> </p>
    <p>{body}</p>
  </div>
}
